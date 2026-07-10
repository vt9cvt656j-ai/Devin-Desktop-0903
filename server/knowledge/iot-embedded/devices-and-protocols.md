# IoT & Embedded Systems

## Device Communication Protocols

### MQTT (Message Queuing Telemetry Transport)
```python
import paho.mqtt.client as mqtt

# QoS Levels:
# 0: At most once (fire and forget) — sensor telemetry
# 1: At least once (ack required) — commands, alerts
# 2: Exactly once (4-step handshake) — billing, critical state changes

client = mqtt.Client(client_id="device_001", protocol=mqtt.MQTTv5)
client.tls_set(ca_certs="ca.pem", certfile="device.pem", keyfile="device.key")
client.username_pw_set("device_001", "token_xyz")

# Topic hierarchy: {org}/{site}/{device_type}/{device_id}/{data_type}
TELEMETRY_TOPIC = "acme/factory-1/sensor/temp-001/reading"
COMMAND_TOPIC = "acme/factory-1/sensor/temp-001/cmd"

def on_message(client, userdata, msg):
    payload = json.loads(msg.payload)
    if msg.topic.endswith('/cmd'):
        handle_command(payload)

client.on_message = on_message
client.connect("mqtt.example.com", 8883)  # TLS port
client.subscribe(COMMAND_TOPIC, qos=1)

# Publish telemetry
reading = {"temperature": 23.5, "humidity": 65, "ts": int(time.time())}
client.publish(TELEMETRY_TOPIC, json.dumps(reading), qos=0)
client.loop_forever()
```

### CoAP (Constrained Application Protocol)
```
- UDP-based (vs MQTT's TCP) — for extremely constrained devices
- RESTful (GET/PUT/POST/DELETE on resources)
- Observe pattern (like MQTT subscribe but resource-oriented)
- DTLS for security
- Use when: battery-powered, <100KB RAM, lossy networks (LoRa, NB-IoT)
```

### Protocol Selection Guide
| Protocol | Transport | Use Case | RAM | Battery |
|----------|-----------|----------|-----|---------|
| MQTT | TCP | Reliable telemetry, commands | >64KB | Medium |
| CoAP | UDP | Constrained sensors, observe | <32KB | Excellent |
| HTTP/REST | TCP | Config, OTA, non-real-time | >128KB | Poor |
| WebSocket | TCP | Real-time dashboard, bidirectional | >128KB | Poor |
| BLE | - | Short-range, wearables, beacons | <16KB | Excellent |
| LoRaWAN | - | Long-range (10km+), low data rate | <32KB | Excellent |

## Device Provisioning & Security

### Zero-Touch Provisioning Flow
```
1. Device boots with factory certificate (unique per device)
2. Connects to provisioning service (bootstrap endpoint)
3. Presents device certificate + attestation
4. Service validates against manufacturing DB
5. Issues operational credentials (X.509 cert or token)
6. Device connects to production MQTT broker
7. Receives initial configuration (shadow/twin)
```

### AWS IoT Device Shadow
```json
{
  "state": {
    "desired": {
      "firmware_version": "2.1.0",
      "reporting_interval_sec": 60,
      "led_color": "green"
    },
    "reported": {
      "firmware_version": "2.0.0",
      "reporting_interval_sec": 300,
      "battery_pct": 72,
      "led_color": "red"
    }
  },
  "metadata": {
    "desired": { "firmware_version": { "timestamp": 1706000000 } },
    "reported": { "firmware_version": { "timestamp": 1705900000 } }
  }
}
// Delta = desired - reported → device receives only what needs to change
```

### Security Rules (Embedded)
```
NEVER: hardcode secrets in firmware (extractable via JTAG/flash dump)
NEVER: use HTTP (always TLS/DTLS, even on internal networks)
NEVER: trust device-side validation alone (server must validate all inputs)
NEVER: allow unsigned firmware updates (use Ed25519 or ECDSA)
ALWAYS: unique credentials per device (compromise one ≠ compromise all)
ALWAYS: implement secure boot chain (bootloader → firmware signature verification)
ALWAYS: support remote credential rotation
ALWAYS: store secrets in secure element / hardware keystore (ATECC608, TPM)
```

## Firmware Architecture

### FreeRTOS Task Pattern
```c
#include "FreeRTOS.h"
#include "task.h"
#include "queue.h"
#include "semphr.h"

// Task priorities (higher = more urgent)
#define PRIORITY_SENSOR    2
#define PRIORITY_COMMS     3
#define PRIORITY_WATCHDOG  4

static QueueHandle_t sensorQueue;
static SemaphoreHandle_t i2cMutex;

void sensorTask(void *pvParams) {
    TickType_t lastWake = xTaskGetTickCount();
    SensorReading reading;

    for (;;) {
        // Take I2C bus mutex
        if (xSemaphoreTake(i2cMutex, pdMS_TO_TICKS(100)) == pdTRUE) {
            reading.temperature = readI2CSensor(TEMP_ADDR);
            reading.humidity = readI2CSensor(HUM_ADDR);
            reading.timestamp = xTaskGetTickCount();
            xSemaphoreGive(i2cMutex);

            // Send to comms task via queue
            xQueueSend(sensorQueue, &reading, pdMS_TO_TICKS(50));
        }

        // Fixed interval (don't drift)
        vTaskDelayUntil(&lastWake, pdMS_TO_TICKS(1000));
    }
}

void commsTask(void *pvParams) {
    SensorReading reading;
    char buffer[256];

    for (;;) {
        if (xQueueReceive(sensorQueue, &reading, portMAX_DELAY) == pdTRUE) {
            snprintf(buffer, sizeof(buffer),
                "{\"temp\":%.1f,\"hum\":%.1f,\"ts\":%lu}",
                reading.temperature, reading.humidity, reading.timestamp);
            mqtt_publish(TOPIC, buffer, QOS_0);
        }
    }
}

int main(void) {
    hardware_init();
    i2cMutex = xSemaphoreCreateMutex();
    sensorQueue = xQueueCreate(10, sizeof(SensorReading));

    xTaskCreate(sensorTask, "Sensor", 512, NULL, PRIORITY_SENSOR, NULL);
    xTaskCreate(commsTask, "Comms", 1024, NULL, PRIORITY_COMMS, NULL);

    vTaskStartScheduler();
    for (;;) {}  // should never reach here
}
```

### Low-Power Design
```c
// Sleep modes (ESP32 example)
typedef enum {
    POWER_ACTIVE,      // full CPU, WiFi, BLE — 160-240mA
    POWER_MODEM_SLEEP, // CPU on, radio off — 20mA
    POWER_LIGHT_SLEEP, // CPU paused, RAM retained — 0.8mA
    POWER_DEEP_SLEEP,  // only RTC + ULP — 10μA
} PowerMode;

void enterDeepSleep(uint32_t wakeup_sec) {
    // Save state to RTC memory (survives deep sleep)
    rtc_data.boot_count++;
    rtc_data.last_reading = current_reading;

    // Configure wakeup source
    esp_sleep_enable_timer_wakeup(wakeup_sec * 1000000ULL);
    esp_sleep_enable_ext0_wakeup(GPIO_NUM_33, 0);  // button wakeup

    // Disconnect everything cleanly
    mqtt_disconnect();
    wifi_disconnect();

    esp_deep_sleep_start();
    // execution stops here, resumes from app_main() on wakeup
}

// Battery life estimation:
// Capacity: 3000mAh
// Active (sending): 150mA × 2s = 0.083mAh per transmission
// Sleep: 10μA × 598s = 0.00166mAh
// Per cycle (10min): ~0.085mAh
// Battery life: 3000 / 0.085 ÷ 6 ÷ 24 = ~245 days
```

## OTA (Over-The-Air) Updates

### Update Flow
```
1. Device checks for update (periodic poll or server push via shadow)
2. Download firmware image in chunks (resume on failure)
3. Verify signature (Ed25519 of SHA-256 hash)
4. Write to inactive partition (A/B scheme)
5. Set boot flag to new partition
6. Reboot → new firmware runs
7. Self-test (connectivity + sensor check)
8. If self-test passes → mark as good (confirm boot)
9. If self-test fails → watchdog reboot → bootloader falls back to previous partition
```

### A/B Partition Layout
```
Flash Layout (4MB example):
┌──────────────────┐ 0x000000
│ Bootloader (64K) │ — verifies partition signature before booting
├──────────────────┤ 0x010000
│ Partition A (1.5M)│ — currently running firmware
├──────────────────┤ 0x190000
│ Partition B (1.5M)│ — OTA target (inactive)
├──────────────────┤ 0x310000
│ NVS (256K)       │ — non-volatile storage (config, WiFi creds)
├──────────────────┤ 0x350000
│ OTA Data (8K)    │ — which partition to boot + rollback counter
└──────────────────┘
```

## Time-Series Data Pipeline

### Ingestion Architecture
```
Devices → MQTT Broker → Stream Processor → Time-Series DB → Dashboard
                             ↓
                        Rules Engine → Alerts
                             ↓
                        Cold Storage (S3/GCS)
```

### Data Schema (TimescaleDB / InfluxDB)
```sql
-- TimescaleDB (PostgreSQL extension)
CREATE TABLE sensor_readings (
    time TIMESTAMPTZ NOT NULL,
    device_id TEXT NOT NULL,
    metric TEXT NOT NULL,      -- 'temperature', 'humidity', 'pressure'
    value DOUBLE PRECISION NOT NULL,
    quality INT DEFAULT 0      -- 0=good, 1=uncertain, 2=bad
);
SELECT create_hypertable('sensor_readings', 'time');

-- Continuous aggregate (pre-computed rollups)
CREATE MATERIALIZED VIEW sensor_hourly
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', time) AS bucket,
    device_id,
    metric,
    AVG(value) AS avg_value,
    MIN(value) AS min_value,
    MAX(value) AS max_value,
    COUNT(*) AS sample_count
FROM sensor_readings
GROUP BY bucket, device_id, metric;

-- Retention policy: raw=30 days, hourly=1 year, daily=forever
SELECT add_retention_policy('sensor_readings', INTERVAL '30 days');
```

### Alert Rules Engine
```python
ALERT_RULES = [
    {
        'name': 'high_temperature',
        'condition': lambda reading: reading['metric'] == 'temperature' and reading['value'] > 85,
        'severity': 'critical',
        'cooldown_sec': 300,
    },
    {
        'name': 'device_offline',
        'condition': lambda device: (time.time() - device['last_seen']) > 600,
        'severity': 'warning',
        'cooldown_sec': 3600,
    },
    {
        'name': 'battery_low',
        'condition': lambda reading: reading['metric'] == 'battery_pct' and reading['value'] < 10,
        'severity': 'warning',
        'cooldown_sec': 86400,
    },
]
```
