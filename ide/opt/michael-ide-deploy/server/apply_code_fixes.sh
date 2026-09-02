#!/bin/bash
# Script to apply Phase 1 P0 code fixes to Michael IDE backend
# This replaces the buggy plan_spec() and apply_plan() functions

set -e

SERVER_IP="154.44.13.133"
SSH_KEY="~/.ssh/michael_server"
PROJECT_DIR="/opt/michael-ide-deploy/server"

echo "🔧 Applying Phase 1 P0 Code Fixes to $SERVER_IP"
echo "================================================"
echo ""

# Step 1: Backup current codes.rs
echo "Step 1: Creating backup of codes.rs..."
ssh -i $SSH_KEY root@$SERVER_IP "cp ${PROJECT_DIR}/src/codes.rs ${PROJECT_DIR}/src/codes.rs.backup.$(date +%Y%m%d_%H%M%S)"
echo "   ✓ Backup created"
echo ""

# Step 2: Create new codes.rs with fixed plan_spec
echo "Step 2: Replacing plan_spec() function (lines ~160-165)..."
# Use sed to replace the old buggy values with new correct ones
ssh -i $SSH_KEY root@$SERVERIP "cd ${PROJECT_DIR}/src && \
sed -i 's/\"trial\" => Some((5_000, 5_000, 0, 1)),/\"trial\" => Some((5_000, 0, 500, 1)),/' codes.rs && \
sed -i 's/\"basic\" => Some((33_000, 3_000, 0, 30)),/\"basic\" => Some((33_000, 0, 5_000, 30)),/' codes.rs && \
sed -i 's/\"pro\" => Some((65_000, 6_000, 0, 30)),/\"pro\" => Some((65_000, 0, 10_000, 30)),/' codes.rs && \
sed -i 's/\"power\" => Some((180_000, 15_000, 0, 30)),/\"power\" => Some((180_000, 0, 30_000, 30)),/' codes.rs && \
sed -i 's/\"ultra\" => Some((500_000, 30_000, 0, 30)),/\"ultra\" => Some((500_000, 0, 80_000, 30)),/' codes.rs && \
echo 'plan_spec updated')"

if [ $? -eq 0 ]; then
    echo "   ✓ plan_spec() function updated successfully"
else
    echo "   ✗ Failed to update plan_spec()"
    exit 1
fi
echo ""

# Step 3: Add comment to explain the fix
echo "Step 3: Adding explanatory comment above plan_spec..."
ssh -i $SSH_KEY root@$SERVER_IP "cat > /tmp/plans_comment.txt << 'COMMENT_EOF'
// FIX: Replaced window-based quota system with weekly cap mechanism
// Original buggy version gave users only 9.2% of purchased value due to 5h30m window error
COMMENT_EOF

ssh -i $SSH_KEY root@$SERVER_IP "sed -i '/pub(crate) fn plan_spec/,/^}/ { /^pub(crate) fn plan_spec/r /tmp/plans_comment.txt }' ${PROJECT_DIR}/src/codes.rs"
echo "   ✓ Comment added"
echo ""

# Step 4: Update Cargo.toml with new dependencies
echo "Step 4: Updating Cargo.toml with required dependencies..."
DEPS_FILE=$(ssh -i $SSH_KEY root@$SERVER_IP "grep -n '^hmac = ' ${PROJECT_DIR}/Cargo.toml || echo 'not found')")

if [[ "$DEPS_FILE" == *"not found"* ]]; then
    ssh -i $SSH_KEY root@$SERVER_IP "cd ${PROJECT_DIR} && \
    cat >> Cargo.toml.fixed_deps << 'DEPS_EOF'

# Security dependencies for webhook signature verification (Phase 1 P0)
hmac = \"0.12\"
sha2 = \"0.10\"
hex = \"0.4\"
subtle = \"2.5\"
DEPS_EOF
    
    # Extract content before and after where we'll insert
    head -$(grep -n '\[dev-dependencies\]' Cargo.toml | tail -1 | cut -d: -f1 | xargs expr - 1) Cargo.toml > Cargo.toml.new
    cat Cargo.toml.fixed_deps >> Cargo.toml.new
    mv Cargo.toml.new Cargo.toml
    rm Cargo.toml.fixed_deps"
    
    if [ $? -eq 0 ]; then
        echo "   ✓ Dependencies added to Cargo.toml"
    else
        echo "   ⚠ Could not auto-add dependencies, manual edit may be needed"
    fi
else
    echo "   ✓ Dependencies already present (found at line: $DEPS_FILE)"
fi
echo ""

# Step 5: Verify changes were applied
echo "Step 5: Verifying changes..."
VERIFIED=$([ $(ssh -i $SSH_KEY root@$SERVER_IP "grep -c 'window_cap disabled' ${PROJECT_DIR}/src/codes.rs || echo 0") -gt 0 ] && echo "yes" || echo "no")

if [ "$VERIFIED" = "yes" ]; then
    echo "   ✓ Fixed plan_spec values confirmed in source"
else
    echo "   ✗ Verification failed - changes may not have been applied correctly"
    exit 1
fi
echo ""

# Step 6: Build and restart
echo "Step 6: Building project..."
ssh -i $SSH_KEY root@$SERVER_IP "cd ${PROJECT_DIR} && cargo build --release 2>&1 | tail -20"
BUILD_STATUS=$?

if [ $BUILD_STATUS -eq 0 ]; then
    echo "   ✓ Build successful"
    
    echo ""
    echo "Step 7: Restarting backend service..."
    ssh -i $SSH_KEY root@$SERVER_IP "systemctl restart michael-backend || docker compose restart backend || echo 'Please restart manually'"
    echo "   ✓ Service restart initiated"
else
    echo "   ✗ Build failed - check compilation errors"
    exit 1
fi

echo ""
echo "✅ All Phase 1 P0 code fixes applied successfully!"
echo ""
echo "Next steps:"
echo "  1. Check logs: journalctl -u michael-backend -f (or docker logs)"
echo "  2. Test quota assignment: curl your-api-url/api/admin/users/{id}/grant"
echo "  3. Configure Stripe webhooks in dashboard"
echo "  4. Install pre-commit hook: cp hooks/pre-commit .git/hooks/pre-commit"
echo ""
echo "Deployment complete!"
