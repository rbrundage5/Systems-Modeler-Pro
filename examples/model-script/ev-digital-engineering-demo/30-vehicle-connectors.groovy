// Systems-Modeler-Pro PR57 EV demo module: 30-vehicle-connectors.groovy.
// Import modules in numeric order. Shared source namespace + stable External IDs make reapply idempotent.
modelScript('''
{
  "source_namespace":"demo:ev-digital-engineering:v3",
  "operations": [
    {"op":"connector","external_id":"CONN_HV","context":"ext:VEH","kind":"Assembly","source_path":["P_ESS","ESS_HV"],"target_path":["P_PT","PT_HV"],"name":"tractionHighVoltage","documentation":""},
    {"op":"connector","external_id":"CONN_HMI_CTRL","context":"ext:VEH","kind":"Assembly","source_path":["P_HMI","HMI_DRIVER"],"target_path":["P_CTRL","CTRL_DRIVER"],"name":"driverCommands","documentation":""},
    {"op":"connector","external_id":"CONN_CTRL_PT","context":"ext:VEH","kind":"Assembly","source_path":["P_CTRL","CTRL_PT"],"target_path":["P_PT","PT_CAN"],"name":"powertrainControl","documentation":""},
    {"op":"connector","external_id":"CONN_CTRL_ESS","context":"ext:VEH","kind":"Assembly","source_path":["P_CTRL","CTRL_ESS"],"target_path":["P_ESS","ESS_CAN"],"name":"energyStorageControl","documentation":""},
    {"op":"connector","external_id":"CONN_SENS_CTRL","context":"ext:VEH","kind":"Assembly","source_path":["P_SENS","SENS_CAN"],"target_path":["P_CTRL","CTRL_SENSOR"],"name":"sensorTelemetry","documentation":""},
    {"op":"connector","external_id":"CONN_CTRL_BRAKE","context":"ext:VEH","kind":"Assembly","source_path":["P_CTRL","CTRL_BRAKE"],"target_path":["P_BRAKES","BRAKE_CMD"],"name":"brakeControl","documentation":""},
    {"op":"connector","external_id":"CONN_CTRL_THERM","context":"ext:VEH","kind":"Assembly","source_path":["P_CTRL","CTRL_THERM"],"target_path":["P_TMS","TMS_CMD"],"name":"thermalControl","documentation":""},
    {"op":"connector","external_id":"CONN_CHARGE_DELEG","context":"ext:VEH","kind":"Delegation","source_path":["EV_CHARGE"],"target_path":["P_ESS","ESS_CHARGE"],"name":"chargingBoundaryDelegation","documentation":""},
    {"op":"connector","external_id":"CONN_SERVICE_DELEG","context":"ext:VEH","kind":"Delegation","source_path":["EV_SERVICE"],"target_path":["P_CTRL","CTRL_SERVICE"],"name":"serviceBoundaryDelegation","documentation":""},
    {"op":"connector","external_id":"CONN_INV_MOTOR","context":"ext:PT","kind":"Assembly","source_path":["P_INV","INV_MOTOR"],"target_path":["P_MOTOR","MOTOR_POWER"],"name":"inverterMotorPower","documentation":""},
    {"op":"connector","external_id":"CONN_CHARGER_BAT","context":"ext:ESS","kind":"Assembly","source_path":["P_CHARGER","CHARGER_BAT"],"target_path":["P_BAT","BAT_CHARGE"],"name":"batteryChargePower","documentation":""},
    {"op":"connector","external_id":"CONN_THERM_PUMP","context":"ext:TMS","kind":"Assembly","source_path":["P_THERM_CTRL","THERM_CAN"],"target_path":["P_PUMP","PUMP_CAN"],"name":"coolingCommand","documentation":""},
    {"op":"connector","external_id":"CONN_BRAKE_ACT","context":"ext:BRAKES","kind":"Assembly","source_path":["P_BRAKE_CTRL","BRAKE_CTRL_CMD"],"target_path":["P_BRAKE_ACT","BRAKE_ACT_CMD"],"name":"actuatorCommand","documentation":""},
    {"op":"item_flow","external_id":"FLOW_HV","connector":"handle:CONN_HV","source_path":["P_ESS","ESS_HV"],"target_path":["P_PT","PT_HV"],"conveyed_items":["ext:DT_HV_POWER"],"name":"High Voltage Availability","documentation":""},
    {"op":"item_flow","external_id":"FLOW_DRIVER","connector":"handle:CONN_HMI_CTRL","source_path":["P_HMI","HMI_DRIVER"],"target_path":["P_CTRL","CTRL_DRIVER"],"conveyed_items":["ext:DT_DRIVER_CMD"],"name":"Driver Commands","documentation":""},
    {"op":"item_flow","external_id":"FLOW_PT_CMD","connector":"handle:CONN_CTRL_PT","source_path":["P_CTRL","CTRL_PT"],"target_path":["P_PT","PT_CAN"],"conveyed_items":["ext:DT_CAN_MSG"],"name":"Powertrain Torque Command","documentation":""},
    {"op":"item_flow","external_id":"FLOW_ESS_CMD","connector":"handle:CONN_CTRL_ESS","source_path":["P_CTRL","CTRL_ESS"],"target_path":["P_ESS","ESS_CAN"],"conveyed_items":["ext:DT_CAN_MSG"],"name":"Energy Storage Commands","documentation":""},
    {"op":"item_flow","external_id":"FLOW_SENSOR","connector":"handle:CONN_SENS_CTRL","source_path":["P_SENS","SENS_CAN"],"target_path":["P_CTRL","CTRL_SENSOR"],"conveyed_items":["ext:DT_CAN_MSG"],"name":"Sensor Telemetry","documentation":""},
    {"op":"item_flow","external_id":"FLOW_BRAKE","connector":"handle:CONN_CTRL_BRAKE","source_path":["P_CTRL","CTRL_BRAKE"],"target_path":["P_BRAKES","BRAKE_CMD"],"conveyed_items":["ext:DT_BRAKE_CMD"],"name":"Brake Command","documentation":""},
    {"op":"item_flow","external_id":"FLOW_THERM","connector":"handle:CONN_CTRL_THERM","source_path":["P_CTRL","CTRL_THERM"],"target_path":["P_TMS","TMS_CMD"],"conveyed_items":["ext:DT_THERMAL_CMD"],"name":"Thermal Control","documentation":""},
    {"op":"item_flow","external_id":"FLOW_CHARGE","connector":"handle:CONN_CHARGE_DELEG","source_path":["EV_CHARGE"],"target_path":["P_ESS","ESS_CHARGE"],"conveyed_items":["ext:DT_CHARGE_POWER"],"name":"Charging Request","documentation":""},
    {"op":"item_flow","external_id":"FLOW_SERVICE","connector":"handle:CONN_SERVICE_DELEG","source_path":["EV_SERVICE"],"target_path":["P_CTRL","CTRL_SERVICE"],"conveyed_items":["ext:DT_SERVICE_MSG"],"name":"Service Diagnostics","documentation":""},
    {"op":"item_flow","external_id":"FLOW_INV_MOTOR","connector":"handle:CONN_INV_MOTOR","source_path":["P_INV","INV_MOTOR"],"target_path":["P_MOTOR","MOTOR_POWER"],"conveyed_items":["ext:DT_HV_POWER"],"name":"Motor Electrical Power","documentation":""},
    {"op":"item_flow","external_id":"FLOW_THERM_PUMP","connector":"handle:CONN_THERM_PUMP","source_path":["P_THERM_CTRL","THERM_CAN"],"target_path":["P_PUMP","PUMP_CAN"],"conveyed_items":["ext:DT_THERMAL_CMD"],"name":"Pump Cooling Command","documentation":""},
    {"op":"item_flow","external_id":"FLOW_BRAKE_ACT","connector":"handle:CONN_BRAKE_ACT","source_path":["P_BRAKE_CTRL","BRAKE_CTRL_CMD"],"target_path":["P_BRAKE_ACT","BRAKE_ACT_CMD"],"conveyed_items":["ext:DT_BRAKE_CMD"],"name":"Brake Actuator Command","documentation":""}
  ]
}
''')
