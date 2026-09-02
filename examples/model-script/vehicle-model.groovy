// Systems-Modeler-Pro PR51 bounded Groovy-compatible model script.
// The Groovy wrapper is only a transport boundary; the triple-quoted payload
// is compiled into Rust ModelBuildPlan operations and native presentation services.
modelScript('''
{
  "source_namespace": "groovy:vehicle-example",
  "operations": [
    {"op":"element","external_id":"PKG","kind":"Package","name":"Vehicle","owner":"$root"},
    {"op":"element","external_id":"REAL","kind":"PrimitiveType","name":"Real","owner":"handle:PKG"},
    {"op":"element","external_id":"VEH","kind":"Block","name":"Vehicle","owner":"handle:PKG"},
    {"op":"element","external_id":"CTRL","kind":"Block","name":"Controller","owner":"handle:PKG"},
    {"op":"element","external_id":"BAT","kind":"Block","name":"Battery","owner":"handle:PKG"},
    {"op":"element","external_id":"CONTROLLER","kind":"PartProperty","name":"controller","owner":"handle:VEH","type_ref":"handle:CTRL"},
    {"op":"element","external_id":"BATTERY","kind":"PartProperty","name":"battery","owner":"handle:VEH","type_ref":"handle:BAT"},
    {"op":"element","external_id":"OP_START","kind":"Operation","name":"start","owner":"handle:CTRL"},
    {"op":"element","external_id":"START_MODE","kind":"Parameter","name":"mode","owner":"handle:OP_START","type_ref":"handle:REAL","parameter_direction":"In"},
    {"op":"element","external_id":"SIG_STARTED","kind":"Signal","name":"Started","owner":"handle:PKG"},
    {"op":"element","external_id":"REC_STARTED","kind":"Reception","name":"started","owner":"handle:CTRL","type_ref":"handle:SIG_STARTED"},
    {"op":"element","external_id":"REQ_START","kind":"Requirement","name":"StartRequirement","owner":"handle:PKG","requirement_id":"REQ-001","requirement_text":"The vehicle shall support commanded startup."},
    {"op":"element","external_id":"TC_START","kind":"TestCase","name":"VerifyStart","owner":"handle:PKG"},
    {"op":"element","external_id":"DRIVER","kind":"Actor","name":"Driver","owner":"handle:PKG"},
    {"op":"element","external_id":"UC_START","kind":"UseCase","name":"StartVehicle","owner":"handle:PKG","extension_points":["startup complete"]},
    {"op":"element","external_id":"CB_FORCE","kind":"ConstraintBlock","name":"ForceEquation","owner":"handle:PKG"},
    {"op":"element","external_id":"CP_M","kind":"ConstraintParameter","name":"m","owner":"handle:CB_FORCE","type_ref":"handle:REAL"},
    {"op":"element","external_id":"MASS","kind":"ValueProperty","name":"mass","owner":"handle:VEH","type_ref":"handle:REAL","default_value":"0"},
    {"op":"element","external_id":"FORCE_EQ","kind":"ConstraintProperty","name":"forceEquation","owner":"handle:VEH","type_ref":"handle:CB_FORCE"},

    {"op":"relationship","external_id":"VEH_CTRL","kind":"Association","source":"handle:VEH","target":"handle:CTRL","owner":"handle:PKG","name":"controllerType"},
    {"op":"relationship","external_id":"SAT_REQ","kind":"Satisfy","source":"handle:VEH","target":"handle:REQ_START","owner":"handle:PKG"},
    {"op":"relationship","external_id":"UC_ASSOC","kind":"Association","source":"handle:DRIVER","target":"handle:UC_START","owner":"handle:PKG"},
    {"op":"relationship","external_id":"PKG_DEP","kind":"Dependency","source":"handle:VEH","target":"handle:CTRL","owner":"handle:PKG"},
    {"op":"connector","external_id":"CTRL_LINK","context":"handle:VEH","kind":"Assembly","source_path":["CONTROLLER"],"target_path":["BATTERY"],"name":"controlPower"},
    {"op":"item_flow","external_id":"STARTED_FLOW","connector":"handle:CTRL_LINK","source_path":["CONTROLLER"],"target_path":["BATTERY"],"conveyed_items":["handle:SIG_STARTED"],"name":"startedFlow"},

    {"op":"activity","external_id":"ACT_START","name":"StartVehicle","owner":"handle:PKG","context":"handle:VEH"},
    {"op":"activity_node","external_id":"ACT_INIT","activity":"handle:ACT_START","name":"Initial","node":{"kind":"initial"}},
    {"op":"activity_node","external_id":"ACT_CALL","activity":"handle:ACT_START","name":"Start Controller","node":{"kind":"call_operation","operation":"handle:OP_START"}},
    {"op":"activity_node","external_id":"ACT_SIGNAL","activity":"handle:ACT_START","name":"Send Started","node":{"kind":"send_signal","signal":"handle:SIG_STARTED"}},
    {"op":"activity_node","external_id":"ACT_FINAL","activity":"handle:ACT_START","name":"Complete","node":{"kind":"activity_final"}},
    {"op":"activity_edge","external_id":"ACT_E1","activity":"handle:ACT_START","name":"","kind":"ControlFlow","source":"handle:ACT_INIT","target":"handle:ACT_CALL"},
    {"op":"activity_edge","external_id":"ACT_E2","activity":"handle:ACT_START","name":"","kind":"ControlFlow","source":"handle:ACT_CALL","target":"handle:ACT_SIGNAL"},
    {"op":"activity_edge","external_id":"ACT_E3","activity":"handle:ACT_START","name":"","kind":"ControlFlow","source":"handle:ACT_SIGNAL","target":"handle:ACT_FINAL"},

    {"op":"state_machine","external_id":"SM_MODES","name":"VehicleModes","context":"handle:VEH"},
    {"op":"region","external_id":"SM_REGION","name":"Modes","state_machine":"handle:SM_MODES"},
    {"op":"vertex","external_id":"SM_INIT","region":"handle:SM_REGION","name":"Initial","vertex":{"kind":"pseudostate","pseudostate":"Initial"}},
    {"op":"vertex","external_id":"SM_ON","region":"handle:SM_REGION","name":"On","vertex":{"kind":"state"}},
    {"op":"vertex","external_id":"SM_FINAL","region":"handle:SM_REGION","name":"Complete","vertex":{"kind":"final_state"}},
    {"op":"transition","external_id":"SM_T1","region":"handle:SM_REGION","source":"handle:SM_INIT","target":"handle:SM_ON"},
    {"op":"transition","external_id":"SM_T2","region":"handle:SM_REGION","source":"handle:SM_ON","target":"handle:SM_FINAL","trigger":{"kind":"signal","signal":"handle:SIG_STARTED"}},

    {"op":"interaction","external_id":"INT_STARTUP","name":"StartupSequence","context":"handle:VEH"},
    {"op":"lifeline","external_id":"LL_BAT","interaction":"handle:INT_STARTUP","name":"battery","represented_path":["handle:BATTERY"]},
    {"op":"lifeline","external_id":"LL_CTRL","interaction":"handle:INT_STARTUP","name":"controller","represented_path":["handle:CONTROLLER"]},
    {"op":"occurrence","external_id":"OCC_SEND","interaction":"handle:INT_STARTUP","lifeline":"handle:LL_BAT","order":1},
    {"op":"occurrence","external_id":"OCC_RECV","interaction":"handle:INT_STARTUP","lifeline":"handle:LL_CTRL","order":2},
    {"op":"occurrence","external_id":"OCC_FINISH","interaction":"handle:INT_STARTUP","lifeline":"handle:LL_CTRL","order":3},
    {"op":"message","external_id":"MSG_START","interaction":"handle:INT_STARTUP","name":"start","sort":"SynchCall","send":"handle:OCC_SEND","receive":"handle:OCC_RECV","signature":{"kind":"operation","operation":"handle:OP_START"},"arguments":["mode=1"]},
    {"op":"execution","external_id":"EXEC_START","interaction":"handle:INT_STARTUP","lifeline":"handle:LL_CTRL","start":"handle:OCC_RECV","finish":"handle:OCC_FINISH","behavior":"handle:OP_START"},

    {"op":"parametric_metadata","element":"handle:CB_FORCE","constraint_expression":"m = 1","quantity_dimension":"M","unit_symbol":"kg","unit_scale_to_base":1.0},
    {"op":"binding","external_id":"BIND","name":"massBinding","owner":"handle:VEH","source":{"role":"handle:MASS"},"target":{"role":"handle:FORCE_EQ","parameter":"handle:CP_M"}}
  ],
  "diagrams": [
    {"external_id":"D_BDD","family":"BDD","name":"Vehicle Structure","owner":"handle:PKG"},
    {"external_id":"D_IBD","family":"IBD","name":"Vehicle Internal Structure","owner":"handle:PKG","context":"handle:VEH"},
    {"external_id":"D_REQ","family":"Requirement","name":"Vehicle Requirements","owner":"handle:PKG"},
    {"external_id":"D_UC","family":"Use Case","name":"Vehicle Use Cases","owner":"handle:PKG","context":"handle:VEH"},
    {"external_id":"D_PKG","family":"Package","name":"Vehicle Package","owner":"handle:PKG"},
    {"external_id":"D_ACT","family":"Activity","name":"Start Vehicle","owner":"handle:PKG","semantic":"handle:ACT_START"},
    {"external_id":"D_SM","family":"State Machine","name":"Vehicle Modes","owner":"handle:PKG","semantic":"handle:SM_MODES"},
    {"external_id":"D_SEQ","family":"Sequence","name":"Startup Sequence","owner":"handle:PKG","semantic":"handle:INT_STARTUP"},
    {"external_id":"D_PAR","family":"Parametric","name":"Vehicle Force","owner":"handle:PKG","context":"handle:VEH"}
  ]
}
''')
