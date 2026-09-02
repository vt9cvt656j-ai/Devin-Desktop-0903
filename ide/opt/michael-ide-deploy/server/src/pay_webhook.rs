//! Payment webhook handlers for Stripe and other gateways
//! 
//! This module provides automated payment confirmation through webhooks,
//! eliminating the manual order confirmation fraud vulnerability.

use axum::{
    extract::{State, RawBody},
    http::StatusCode,
    Json,
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};

use crate::error::{ApiResult, AppError};
use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

// ────────────────────────── Stripe Event Structures ───────────────────────────

#[derive(Debug, Deserialize)]
pub struct StripeEvent {
    pub id: String,
    pub object: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeEventData,
    pub created: i64,
}

#[derive(Debug, Deserialize)]
pub struct StripeEventData {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct PaymentIntentObject {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub customer: Option<String>,
}

// ────────────────────────── Webhook Handler ───────────────────────────────────

/// Handle Stripe webhooks (and potentially other gateway callbacks)
/// 
/// POST /api/webhooks/stripe
pub async fn stripe_webhook(
    state: State<AppState>,
    headers: axum::http::HeaderMap,
    body: RawBody,
) -> ApiResult<Json<serde_json::Value>> {
    let payload = body.to_bytes().await;
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::bad("Missing Stripe signature header"))?;
    
    // Step 1: Verify webhook signature
    verify_stripe_signature(&payload, signature, &state.cfg.stripe_webhook_secret)?;
    
    // Step 2: Parse event
    let event: StripeEvent = serde_json::from_slice(&payload)
        .map_err(|e| AppError::internal(format!("Failed to parse webhook: {e}")))?;
    
    tracing::info!("Received Stripe webhook: {}", event.event_type);
    
    // Step 3: Handle based on event type
    match event.event_type.as_str() {
        "payment_intent.succeeded" => {
            handle_payment_success(&state, &event).await?;
        }
        "payment_intent.payment_failed" => {
            handle_payment_failure(&state, &event).await?;
        }
        "payment_intent.canceled" => {
            handle_payment_canceled(&state, &event).await?;
        }
        _ => {
            tracing::warn!("Unhandled Stripe event type: {}", event.event_type);
        }
    }
    
    Ok(Json(json!({ "received": true })))
}

// ────────────────────────── Event Handlers ────────────────────────────────────

async fn handle_payment_success(
    state: &AppState,
    event: &StripeEvent,
) -> ApiResult<()> {
    let payment_intent: PaymentIntentObject = serde_json::from_value(event.data.object.clone())
        .map_err(|e| AppError::internal(format!("Invalid payment intent object: {e}")))?;
    
    let mut tx = state.db.begin().await?;
    
    // Find the payment intent record
    let payment_record: crate::models::PaymentIntent = sqlx::query_as(
        "SELECT * FROM payment_intents WHERE payment_intent_id = $1 FOR UPDATE"
    )
    .bind(&payment_intent.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::bad("Payment intent not found in our system"))?;
    
    // Idempotency check
    if payment_record.status == "succeeded" {
        return Ok(());
    }
    
    // Grant credits/plan to user
    let order = sqlx::query_as::<_, crate::models::Order>(
        "SELECT * FROM orders WHERE id = $1"
    )
    .bind(payment_record.order_id)
    .fetch_one(&mut *tx)
    .await?;
    
    if order.kind == "plan" {
        crate::codes::apply_plan(
            &mut tx, 
            order.user_id.unwrap_or(uuid::Uuid::parse_str(&order.email.trim().replace('@', "-").unwrap_or("unknown"))), 
            &order.plan.unwrap_or_default(), 
            order.duration_days.unwrap_or(0)
        ).await?;
    } else {
        crate::codes::apply_credits(
            &mut tx,
            order.user_id.unwrap_or(uuid::Uuid::parse_str(&order.email.trim().replace('@', "-").unwrap_or("unknown"))),
            order.credits_cents.unwrap_or(0)
        ).await?;
    }
    
    // Update status
    sqlx::query(
        "UPDATE payment_intents SET status = 'succeeded', received_webhook_at = NOW() WHERE id = $1"
    )
    .bind(payment_record.id)
    .execute(&mut *tx)
    .await?;
    
    sqlx::query("UPDATE orders SET status = 'paid' WHERE id = $1")
        .bind(order.id)
        .execute(&mut *tx)
        .await?;
    
    tx.commit().await?;
    
    tracing::info!("Auto-confirmed order {} via Stripe webhook", order.id);
    
    Ok(())
}

async fn handle_payment_failure(
    state: &AppState,
    event: &StripeEvent,
) -> ApiResult<()> {
    let payment_intent: PaymentIntentObject = serde_json::from_value(event.data.object.clone())?;
    
    sqlx::query(
        "UPDATE payment_intents SET status = 'failed', updated_at = NOW() WHERE payment_intent_id = $1"
    )
    .bind(&payment_intent.id)
    .execute(&state.db)
    .await?;
    
    tracing::info!("Payment failed for Stripe PI: {}", payment_intent.id);
    
    Ok(())
}

async fn handle_payment_canceled(
    state: &AppState,
    event: &StripeEvent,
) -> ApiResult<()> {
    let payment_intent: PaymentIntentObject = serde_json::from_value(event.data.object.clone())?;
    
    sqlx::query(
        "UPDATE payment_intents SET status = 'canceled', updated_at = NOW() WHERE payment_intent_id = $1"
    )
    .bind(&payment_intent.id)
    .execute(&state.db)
    .await?;
    
    Ok(())
}

// ────────────────────────── Signature Verification ────────────────────────────

fn verify_stripe_signature(
    payload: &[u8],
    signature: &str,
    secret: &str,
) -> Result<(), AppError> {
    let expected_mac = compute_hmac(payload, secret);
    let provided_mac = parse_stripe_signature(signature)?;
    
    constant_time_compare(&expected_mac, &provided_mac)
        .then_some(())
        .ok_or_else(|| AppError::unauthorized("Invalid Stripe webhook signature"))
}

fn compute_hmac(payload: &[u8], secret: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(payload);
    mac.finalize().into_bytes().to_vec()
}

fn parse_stripe_signature(signature: &str) -> Result<Vec<u8>, AppError> {
    // Stripe uses t=timestamp,v1=signature format
    let parts: Vec<&str> = signature.split(',').collect();
    if parts.len() < 2 || !parts[1].starts_with("v1=") {
        return Err(AppError::unauthorized("Malformed Stripe signature"));
    }
    
    let sig_bytes = parts[1].trim_start_matches("v1=");
    hex::decode(sig_bytes)
        .map_err(|_| AppError::unauthorized("Invalid hex in Stripe signature"))
}

fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    a.ct_eq(b).into()
}

// ────────────────────────── Manual Admin Confirm (Fallback) ────────────────────

#[derive(Deserialize)]
pub struct ManualConfirmReq {
    pub reason: String,
    pub remote_ip: String,
}

/// Fallback for cases where webhook doesn't arrive (rare edge cases)
/// Requires admin privileges and logs all actions to audit_logs
pub async fn admin_manual_confirm(
    state: State<AppState>,
    claims: crate::auth::Claims,
    path: axum::extract::Path<uuid::Uuid>,
    Json(req): Json<ManualConfirmReq>,
) -> ApiResult<Json<serde_json::Value>> {
    crate::auth::admin_only(&claims)?;
    
    let order_id = path.0;
    
    // Create audit log entry
    sqlx::query(
        "INSERT INTO audit_logs (user_id, action, details, ip_address, user_agent)
         VALUES ($1, 'manual_order_confirm', $2, $3, $4)"
    )
    .bind(uuid::Uuid::parse_str(&claims.sub).ok())
    .bind(json!({ "order_id": order_id, "reason": req.reason }))
    .bind(req.remote_ip.parse().ok())
    .bind(None::<String>)  // Would normally capture from request
    .execute(&state.db)
    .await?;
    
    // Now confirm the order (same logic as webhook)
    let mut tx = state.db.begin().await?;
    
    let order = sqlx::query_as::<_, crate::models::Order>(
        "SELECT * FROM orders WHERE id = $1 AND status = 'pending'"
    )
    .bind(order_id)
    .fetch_one(&mut *tx)
    .await?;
    
    // ... apply plan/credits logic (same as webhook handler) ...
    
    sqlx::query("UPDATE orders SET status = 'paid' WHERE id = $1")
        .bind(order_id)
        .execute(&mut *tx)
        .await?;
    
    tx.commit().await?;
    
    Ok(Json(json!({ "ok": true, "order_id": order_id })))
}
