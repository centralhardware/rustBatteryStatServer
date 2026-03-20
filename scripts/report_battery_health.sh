#!/bin/bash

SERVER_URL="localhost:8080"
ENDPOINT="/api/battery/health"

DEVICE_ID=$(/usr/sbin/ioreg -rd1 -c IOPlatformExpertDevice | /usr/bin/awk '/IOPlatformUUID/ { print $3; }' | /usr/bin/tr -d '"')

IOREG_DATA=$(/usr/sbin/ioreg -r -c AppleSmartBattery)

CYCLE_COUNT=$(echo "$IOREG_DATA" | /usr/bin/grep '"CycleCount" =' | /usr/bin/awk '{print $3}')
CURRENT_CHARGE=$(echo "$IOREG_DATA" | /usr/bin/grep '"CurrentCapacity" =' | /usr/bin/awk '{print $3}')
MAX_CAPACITY=$(echo "$IOREG_DATA" | /usr/bin/grep '"MaxCapacity" =' | /usr/bin/awk '{print $3}')
TEMPERATURE=$(echo "$IOREG_DATA" | /usr/bin/grep '"Temperature" =' | /usr/bin/awk '{print $3}')
IS_CHARGING=$(echo "$IOREG_DATA" | /usr/bin/grep '"IsCharging" =' | /usr/bin/awk '{print $3}')
DESIGN_CAPACITY_MAH=$(echo "$IOREG_DATA" | /usr/bin/grep '"DesignCapacity" =' | /usr/bin/awk '{print $3}')
MAX_CAPACITY_MAH=$(echo "$IOREG_DATA" | /usr/bin/grep '"AppleRawMaxCapacity" =' | /usr/bin/awk '{print $3}')
VOLTAGE=$(echo "$IOREG_DATA" | /usr/bin/grep '"Voltage" =' | /usr/bin/awk '{print $3}')
CURRENT=$(echo "$IOREG_DATA" | /usr/bin/grep '"InstantAmperage" =' | /usr/bin/awk '{print $3}')
AVG_TIME_TO_EMPTY=$(echo "$IOREG_DATA" | /usr/bin/grep '"AvgTimeToEmpty" =' | /usr/bin/awk '{print $3}')
if [ "$AVG_TIME_TO_EMPTY" = "65535" ]; then AVG_TIME_TO_EMPTY=0; fi
AVG_TIME_TO_FULL=$(echo "$IOREG_DATA" | /usr/bin/grep '"AvgTimeToFull" =' | /usr/bin/awk '{print $3}')
if [ "$AVG_TIME_TO_FULL" = "65535" ]; then AVG_TIME_TO_FULL=0; fi
EXTERNAL_CONNECTED=$(echo "$IOREG_DATA" | /usr/bin/grep '"ExternalConnected" =' | /usr/bin/awk '{print $3}')
FULLY_CHARGED=$(echo "$IOREG_DATA" | /usr/bin/grep '"FullyCharged" =' | /usr/bin/awk '{print $3}')
NOMINAL_CHARGE_CAPACITY=$(echo "$IOREG_DATA" | /usr/bin/grep '"NominalChargeCapacity" =' | /usr/bin/awk '{print $3}')
RAW_CURRENT_CAPACITY=$(echo "$IOREG_DATA" | /usr/bin/grep '"AppleRawCurrentCapacity" =' | /usr/bin/awk '{print $3}')
RAW_BATTERY_VOLTAGE=$(echo "$IOREG_DATA" | /usr/bin/grep '"AppleRawBatteryVoltage" =' | /usr/bin/awk '{print $3}')
VIRTUAL_TEMPERATURE=$(echo "$IOREG_DATA" | /usr/bin/grep '"VirtualTemperature" =' | /usr/bin/awk '{print $3}')
AT_CRITICAL_LEVEL=$(echo "$IOREG_DATA" | /usr/bin/grep '"AtCriticalLevel" =' | /usr/bin/awk '{print $3}')
BATTERY_CELL_DISCONNECT_COUNT=$(echo "$IOREG_DATA" | /usr/bin/grep '"BatteryCellDisconnectCount" =' | /usr/bin/awk '{print $3}')
DESIGN_CYCLE_COUNT=$(echo "$IOREG_DATA" | /usr/bin/grep '"DesignCycleCount9C" =' | /usr/bin/awk '{print $3}')

# Cell voltages from BatteryData (inline dict on one line)
CELL_VOLTAGES=$(echo "$IOREG_DATA" | /usr/bin/sed -n 's/.*"CellVoltage"=(\([^)]*\)).*/\1/p')
CELL_VOLTAGE_1=$(echo "$CELL_VOLTAGES" | /usr/bin/awk -F',' '{gsub(/ /,"",$1); print $1}')
CELL_VOLTAGE_2=$(echo "$CELL_VOLTAGES" | /usr/bin/awk -F',' '{gsub(/ /,"",$2); print $2}')
CELL_VOLTAGE_3=$(echo "$CELL_VOLTAGES" | /usr/bin/awk -F',' '{gsub(/ /,"",$3); print $3}')

# Adapter details (inline dict on one line)
ADAPTER_LINE=$(echo "$IOREG_DATA" | /usr/bin/grep '"AdapterDetails" =')
ADAPTER_WATTS=$(echo "$ADAPTER_LINE" | /usr/bin/sed -n 's/.*"Watts"=\([0-9]*\).*/\1/p')
ADAPTER_NAME=$(echo "$ADAPTER_LINE" | /usr/bin/sed -n 's/.*"Name"="\([^"]*\)".*/\1/p' | /usr/bin/sed 's/^ *//;s/ *$//')
ADAPTER_VOLTAGE=$(echo "$ADAPTER_LINE" | /usr/bin/sed -n 's/.*"AdapterVoltage"=\([0-9]*\).*/\1/p')

HEALTH_PERCENT=$(/usr/sbin/system_profiler SPPowerDataType | /usr/bin/grep "Maximum Capacity" | /usr/bin/awk '{print $3}' | /usr/bin/tr -d '%')

if [ -n "$CURRENT_CHARGE" ] && [ -n "$MAX_CAPACITY" ] && [ "$MAX_CAPACITY" -gt 0 ]; then
    CHARGE_PERCENT=$((CURRENT_CHARGE * 100 / MAX_CAPACITY))
else
    CHARGE_PERCENT=0
fi

if [ -n "$TEMPERATURE" ]; then
    TEMP_CELSIUS=$(echo "scale=2; $TEMPERATURE / 100" | /usr/bin/bc)
else
    TEMP_CELSIUS=0
fi

if [ -n "$VIRTUAL_TEMPERATURE" ]; then
    VIRTUAL_TEMP_CELSIUS=$(echo "scale=2; $VIRTUAL_TEMPERATURE / 100" | /usr/bin/bc)
else
    VIRTUAL_TEMP_CELSIUS=0
fi

if [ "$IS_CHARGING" = "Yes" ]; then
    IS_CHARGING_BOOL="true"
else
    IS_CHARGING_BOOL="false"
fi

if [ "$EXTERNAL_CONNECTED" = "Yes" ]; then
    EXTERNAL_CONNECTED_BOOL="true"
else
    EXTERNAL_CONNECTED_BOOL="false"
fi

if [ "$FULLY_CHARGED" = "Yes" ]; then
    FULLY_CHARGED_BOOL="true"
else
    FULLY_CHARGED_BOOL="false"
fi

if [ "$AT_CRITICAL_LEVEL" = "Yes" ]; then
    AT_CRITICAL_LEVEL_BOOL="true"
else
    AT_CRITICAL_LEVEL_BOOL="false"
fi

# Voltage is already in mV
if [ -z "$VOLTAGE" ]; then
    VOLTAGE=0
fi

if [ -n "$CURRENT" ]; then
    CURRENT=$(/usr/bin/python3 -c "import sys; val = $CURRENT; print(val if val <= 9223372036854775807 else val - 18446744073709551616)")
else
    CURRENT=0
fi

echo "Debug info:"
echo "  Device ID: $DEVICE_ID"
echo "  Cycle Count: $CYCLE_COUNT"
echo "  Health Percent: $HEALTH_PERCENT%"
echo "  Current Charge: $CHARGE_PERCENT%"
echo "  Temperature: ${TEMP_CELSIUS}°C"
echo "  Is Charging: $IS_CHARGING_BOOL"
echo "  Design Capacity: ${DESIGN_CAPACITY_MAH} mAh"
echo "  Max Capacity: ${MAX_CAPACITY_MAH} mAh"
echo "  Voltage: ${VOLTAGE} mV"
echo "  Current: ${CURRENT} mA"
echo "  Avg Time To Empty: ${AVG_TIME_TO_EMPTY:-0} min"
echo "  Avg Time To Full: ${AVG_TIME_TO_FULL:-0} min"
echo "  External Connected: $EXTERNAL_CONNECTED_BOOL"
echo "  Fully Charged: $FULLY_CHARGED_BOOL"
echo "  Nominal Charge Capacity: ${NOMINAL_CHARGE_CAPACITY:-0} mAh"
echo "  Raw Current Capacity: ${RAW_CURRENT_CAPACITY:-0} mAh"
echo "  Raw Battery Voltage: ${RAW_BATTERY_VOLTAGE:-0} mV"
echo "  Virtual Temperature: ${VIRTUAL_TEMP_CELSIUS}°C"
echo "  Cell Voltages: ${CELL_VOLTAGE_1:-0}, ${CELL_VOLTAGE_2:-0}, ${CELL_VOLTAGE_3:-0} mV"
echo "  At Critical Level: $AT_CRITICAL_LEVEL_BOOL"
echo "  Cell Disconnect Count: ${BATTERY_CELL_DISCONNECT_COUNT:-0}"
echo "  Adapter Watts: ${ADAPTER_WATTS:-0} W"
echo "  Adapter Name: ${ADAPTER_NAME:-}"
echo "  Adapter Voltage: ${ADAPTER_VOLTAGE:-0} mV"
echo "  Design Cycle Count: ${DESIGN_CYCLE_COUNT:-0}"

if [ -z "$CYCLE_COUNT" ] || [ -z "$HEALTH_PERCENT" ]; then
    echo "$(/bin/date '+%Y-%m-%d %H:%M:%S') - Error: Failed to get battery information"
    exit 1
fi

JSON_PAYLOAD=$(/bin/cat <<EOF
{
  "deviceId": "$DEVICE_ID",
  "cycleCount": $CYCLE_COUNT,
  "healthPercent": $HEALTH_PERCENT,
  "currentCharge": $CHARGE_PERCENT,
  "temperature": $TEMP_CELSIUS,
  "isCharging": $IS_CHARGING_BOOL,
  "designCapacityMah": ${DESIGN_CAPACITY_MAH:-0},
  "maxCapacityMah": ${MAX_CAPACITY_MAH:-0},
  "voltageMv": ${VOLTAGE:-0},
  "currentMa": ${CURRENT:-0},
  "avgTimeToEmpty": ${AVG_TIME_TO_EMPTY:-0},
  "avgTimeToFull": ${AVG_TIME_TO_FULL:-0},
  "externalConnected": $EXTERNAL_CONNECTED_BOOL,
  "fullyCharged": $FULLY_CHARGED_BOOL,
  "nominalChargeCapacity": ${NOMINAL_CHARGE_CAPACITY:-0},
  "rawCurrentCapacity": ${RAW_CURRENT_CAPACITY:-0},
  "rawBatteryVoltage": ${RAW_BATTERY_VOLTAGE:-0},
  "virtualTemperature": $VIRTUAL_TEMP_CELSIUS,
  "cellVoltage1": ${CELL_VOLTAGE_1:-0},
  "cellVoltage2": ${CELL_VOLTAGE_2:-0},
  "cellVoltage3": ${CELL_VOLTAGE_3:-0},
  "atCriticalLevel": $AT_CRITICAL_LEVEL_BOOL,
  "batteryCellDisconnectCount": ${BATTERY_CELL_DISCONNECT_COUNT:-0},
  "adapterWatts": ${ADAPTER_WATTS:-0},
  "adapterName": "${ADAPTER_NAME:-}",
  "adapterVoltage": ${ADAPTER_VOLTAGE:-0},
  "designCycleCount": ${DESIGN_CYCLE_COUNT:-0}
}
EOF
)

RESPONSE=$(/usr/bin/curl -s -w "\n%{http_code}" -X POST \
    -H "Content-Type: application/json" \
    -d "$JSON_PAYLOAD" \
    "${SERVER_URL}${ENDPOINT}")

HTTP_CODE=$(echo "$RESPONSE" | /usr/bin/tail -n1)
BODY=$(echo "$RESPONSE" | /usr/bin/sed '$d')

if [ "$HTTP_CODE" = "201" ]; then
    echo "$(/bin/date '+%Y-%m-%d %H:%M:%S') - Battery health reported successfully"
    exit 0
else
    echo "$(/bin/date '+%Y-%m-%d %H:%M:%S') - Failed to report battery health. HTTP code: $HTTP_CODE"
    echo "Response: $BODY"
    exit 1
fi
