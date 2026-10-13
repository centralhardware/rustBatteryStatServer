ClickHouse schema for `batteryServer.battery_health`. V1–V11 were applied by Flyway
from the archived Kotlin server (centralhardware/batteryStatServer). This server
doesn't run migrations, so apply new ones by hand, in order and before deploying
the code that needs them. Record each one in `flyway_schema_history`.
