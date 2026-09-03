// Systems-Modeler-Pro PR57 EV demo module: 50-state-machines.groovy.
// Import modules in numeric order. Shared source namespace + stable External IDs make reapply idempotent.
modelScript('''
{
  "source_namespace":"demo:ev-digital-engineering:v3",
  "operations": [
    {"op":"state_machine","external_id":"SM_VEH","name":"VehicleOperatingStates","context":"ext:VEH"},
    {"op":"region","external_id":"SM_VEH_R","name":"Operating","state_machine":"handle:SM_VEH"},
    {"op":"vertex","external_id":"SV_INIT","region":"handle:SM_VEH_R","name":"Initial","vertex":{"kind":"pseudostate","pseudostate":"Initial"}},
    {"op":"vertex","external_id":"SV_OFF","region":"handle:SM_VEH_R","name":"Off","vertex":{"kind":"state","entry":"false"}},
    {"op":"vertex","external_id":"SV_STARTING","region":"handle:SM_VEH_R","name":"Starting","vertex":{"kind":"state","do_activity":"true"}},
    {"op":"vertex","external_id":"SV_READY","region":"handle:SM_VEH_R","name":"Ready","vertex":{"kind":"state","entry":"true"}},
    {"op":"vertex","external_id":"SV_DRIVING","region":"handle:SM_VEH_R","name":"Driving","vertex":{"kind":"state","do_activity":"true"}},
    {"op":"vertex","external_id":"SV_CHARGING","region":"handle:SM_VEH_R","name":"Charging","vertex":{"kind":"state","do_activity":"true"}},
    {"op":"vertex","external_id":"SV_FAULT","region":"handle:SM_VEH_R","name":"Fault","vertex":{"kind":"state","entry":"true"}},
    {"op":"vertex","external_id":"SV_CHOICE","region":"handle:SM_VEH_R","name":"Operational Choice","vertex":{"kind":"pseudostate","pseudostate":"Choice"}},
    {"op":"vertex","external_id":"SV_FINAL","region":"handle:SM_VEH_R","name":"Shutdown Complete","vertex":{"kind":"final_state"}},
    {"op":"transition","external_id":"SV_T0","region":"handle:SM_VEH_R","source":"handle:SV_INIT","target":"handle:SV_OFF"},
    {"op":"transition","external_id":"SV_T1","region":"handle:SM_VEH_R","source":"handle:SV_OFF","target":"handle:SV_STARTING","trigger":{"kind":"signal","signal":"ext:SIG_START"},"effect":"true"},
    {"op":"transition","external_id":"SV_T2","region":"handle:SM_VEH_R","source":"handle:SV_STARTING","target":"handle:SV_READY","trigger":{"kind":"time","expression":"2.0","is_relative":true},"guard":"true"},
    {"op":"transition","external_id":"SV_T3","region":"handle:SM_VEH_R","source":"handle:SV_READY","target":"handle:SV_CHOICE","trigger":{"kind":"call","operation":"ext:OP_TORQUE"}},
    {"op":"transition","external_id":"SV_T4","region":"handle:SM_VEH_R","source":"handle:SV_CHOICE","target":"handle:SV_DRIVING","guard":"true"},
    {"op":"transition","external_id":"SV_T4B","region":"handle:SM_VEH_R","source":"handle:SV_CHOICE","target":"handle:SV_FAULT","guard":"false"},
    {"op":"transition","external_id":"SV_T5","region":"handle:SM_VEH_R","source":"handle:SV_DRIVING","target":"handle:SV_FAULT","trigger":{"kind":"change","expression":"batteryTemperature > limit"}},
    {"op":"transition","external_id":"SV_T6","region":"handle:SM_VEH_R","source":"handle:SV_READY","target":"handle:SV_FAULT","trigger":{"kind":"any_receive"}},
    {"op":"transition","external_id":"SV_T7","region":"handle:SM_VEH_R","source":"handle:SV_READY","target":"handle:SV_CHARGING","trigger":{"kind":"signal","signal":"ext:SIG_CHARGE"}},
    {"op":"transition","external_id":"SV_T8","region":"handle:SM_VEH_R","source":"handle:SV_CHARGING","target":"handle:SV_READY","trigger":{"kind":"signal","signal":"ext:SIG_STOP"}},
    {"op":"transition","external_id":"SV_T9","region":"handle:SM_VEH_R","source":"handle:SV_READY","target":"handle:SV_FINAL","trigger":{"kind":"signal","signal":"ext:SIG_STOP"}},
    {"op":"transition","external_id":"SV_T10","region":"handle:SM_VEH_R","source":"handle:SV_FAULT","target":"handle:SV_FINAL","trigger":{"kind":"call","operation":"ext:OP_STOP"}},
    {"op":"state_machine","external_id":"SM_BAT","name":"BatteryOperatingStates","context":"ext:BAT"},
    {"op":"region","external_id":"SM_BAT_R","name":"Battery Modes","state_machine":"handle:SM_BAT"},
    {"op":"vertex","external_id":"SB_INIT","region":"handle:SM_BAT_R","name":"Initial","vertex":{"kind":"pseudostate","pseudostate":"Initial"}},
    {"op":"vertex","external_id":"SB_IDLE","region":"handle:SM_BAT_R","name":"Idle","vertex":{"kind":"state"}},
    {"op":"vertex","external_id":"SB_DIS","region":"handle:SM_BAT_R","name":"Discharging","vertex":{"kind":"state"}},
    {"op":"vertex","external_id":"SB_CHG","region":"handle:SM_BAT_R","name":"Charging","vertex":{"kind":"state"}},
    {"op":"vertex","external_id":"SB_FAULT","region":"handle:SM_BAT_R","name":"Protected Fault","vertex":{"kind":"state"}},
    {"op":"transition","external_id":"SB_T0","region":"handle:SM_BAT_R","source":"handle:SB_INIT","target":"handle:SB_IDLE"},
    {"op":"transition","external_id":"SB_T1","region":"handle:SM_BAT_R","source":"handle:SB_IDLE","target":"handle:SB_DIS","trigger":{"kind":"signal","signal":"ext:SIG_HV_ENABLE"}},
    {"op":"transition","external_id":"SB_T2","region":"handle:SM_BAT_R","source":"handle:SB_IDLE","target":"handle:SB_CHG","trigger":{"kind":"signal","signal":"ext:SIG_CHARGE"}},
    {"op":"transition","external_id":"SB_T3","region":"handle:SM_BAT_R","source":"handle:SB_DIS","target":"handle:SB_FAULT","trigger":{"kind":"change","expression":"batteryTemperature > limit"}},
    {"op":"transition","external_id":"SB_T4","region":"handle:SM_BAT_R","source":"handle:SB_CHG","target":"handle:SB_IDLE","trigger":{"kind":"any_receive"}}
  ]
}
''')
