// Systems-Modeler-Pro PR57 EV demo module: 21-use-cases-traceability.groovy.
// Import modules in numeric order. Shared source namespace + stable External IDs make reapply idempotent.
modelScript('''
{
  "source_namespace":"demo:ev-digital-engineering:v3",
  "operations": [
    {"op":"element","external_id":"ACTOR_DRIVER","kind":"Actor","name":"Driver","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"ACTOR_TECH","kind":"Actor","name":"ServiceTechnician","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"ACTOR_CHARGER","kind":"Actor","name":"ChargingInfrastructure","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"ACTOR_TEST","kind":"Actor","name":"TestEngineer","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"UC_OPERATE","kind":"UseCase","name":"Operate Vehicle","owner":"ext:PKG_UC","extension_points":["vehicle stopped","fault detected"]},
    {"op":"element","external_id":"UC_START","kind":"UseCase","name":"Start Vehicle","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"UC_DRIVE","kind":"UseCase","name":"Drive Vehicle","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"UC_STOP","kind":"UseCase","name":"Stop Vehicle","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"UC_ACCEL","kind":"UseCase","name":"Accelerate Vehicle","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"UC_BRAKE","kind":"UseCase","name":"Emergency Braking","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"UC_CHARGE","kind":"UseCase","name":"Charge Vehicle","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"UC_DIAG","kind":"UseCase","name":"Run Diagnostics","owner":"ext:PKG_UC"},
    {"op":"element","external_id":"UC_VERIFY","kind":"UseCase","name":"Verify Vehicle","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"A_DRIVER_OPERATE","kind":"Association","source":"handle:ACTOR_DRIVER","target":"handle:UC_OPERATE","owner":"ext:PKG_UC","source_end":{"role_name":"actor","multiplicity":{"lower":1,"upper":1},"navigable":false,"aggregation":"None"},"target_end":{"role_name":"useCase","multiplicity":{"lower":1,"upper":1},"navigable":true,"aggregation":"None"}},
    {"op":"relationship","external_id":"A_DRIVER_CHARGE","kind":"Association","source":"handle:ACTOR_DRIVER","target":"handle:UC_CHARGE","owner":"ext:PKG_UC","source_end":{"role_name":"actor","multiplicity":{"lower":1,"upper":1},"navigable":false,"aggregation":"None"},"target_end":{"role_name":"useCase","multiplicity":{"lower":1,"upper":1},"navigable":true,"aggregation":"None"}},
    {"op":"relationship","external_id":"A_TECH_DIAG","kind":"Association","source":"handle:ACTOR_TECH","target":"handle:UC_DIAG","owner":"ext:PKG_UC","source_end":{"role_name":"actor","multiplicity":{"lower":1,"upper":1},"navigable":false,"aggregation":"None"},"target_end":{"role_name":"useCase","multiplicity":{"lower":1,"upper":1},"navigable":true,"aggregation":"None"}},
    {"op":"relationship","external_id":"A_CHARGER_CHARGE","kind":"Association","source":"handle:ACTOR_CHARGER","target":"handle:UC_CHARGE","owner":"ext:PKG_UC","source_end":{"role_name":"actor","multiplicity":{"lower":1,"upper":1},"navigable":false,"aggregation":"None"},"target_end":{"role_name":"useCase","multiplicity":{"lower":1,"upper":1},"navigable":true,"aggregation":"None"}},
    {"op":"relationship","external_id":"A_TEST_VERIFY","kind":"Association","source":"handle:ACTOR_TEST","target":"handle:UC_VERIFY","owner":"ext:PKG_UC","source_end":{"role_name":"actor","multiplicity":{"lower":1,"upper":1},"navigable":false,"aggregation":"None"},"target_end":{"role_name":"useCase","multiplicity":{"lower":1,"upper":1},"navigable":true,"aggregation":"None"}},
    {"op":"relationship","external_id":"INC_START","kind":"Include","source":"handle:UC_OPERATE","target":"handle:UC_START","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"INC_DRIVE","kind":"Include","source":"handle:UC_OPERATE","target":"handle:UC_DRIVE","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"INC_STOP","kind":"Include","source":"handle:UC_OPERATE","target":"handle:UC_STOP","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"INC_ACCEL","kind":"Include","source":"handle:UC_DRIVE","target":"handle:UC_ACCEL","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"INC_VERIFY_DIAG","kind":"Include","source":"handle:UC_VERIFY","target":"handle:UC_DIAG","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"EXT_BRAKE","kind":"Extend","source":"handle:UC_BRAKE","target":"handle:UC_OPERATE","owner":"ext:PKG_UC","extension_condition":"brake request requires immediate deceleration","extension_location":"vehicle stopped"},
    {"op":"relationship","external_id":"EXT_DIAG","kind":"Extend","source":"handle:UC_DIAG","target":"handle:UC_OPERATE","owner":"ext:PKG_UC","extension_condition":"fault or service request present","extension_location":"fault detected"},
    {"op":"relationship","external_id":"GEN_ACCEL_DRIVE","kind":"Generalization","source":"handle:UC_ACCEL","target":"handle:UC_DRIVE","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"TR_UC_START","kind":"Trace","source":"handle:UC_START","target":"ext:REQ_SYS_START","owner":"ext:PKG_REQ"},
    {"op":"relationship","external_id":"TR_UC_DRIVE","kind":"Trace","source":"handle:UC_DRIVE","target":"ext:REQ_SYS_PROP","owner":"ext:PKG_REQ"},
    {"op":"relationship","external_id":"TR_UC_CHARGE","kind":"Trace","source":"handle:UC_CHARGE","target":"ext:REQ_SYS_CHARGE","owner":"ext:PKG_REQ"},
    {"op":"relationship","external_id":"TR_UC_DIAG","kind":"Trace","source":"handle:UC_DIAG","target":"ext:REQ_SYS_DIAG","owner":"ext:PKG_REQ"},
    {"op":"relationship","external_id":"TR_UC_BRAKE","kind":"Trace","source":"handle:UC_BRAKE","target":"ext:REQ_SAFE_BRAKE","owner":"ext:PKG_REQ"},
    {"op":"relationship","external_id":"TR_UC_VERIFY","kind":"Trace","source":"handle:UC_VERIFY","target":"ext:REQ_VER_START","owner":"ext:PKG_REQ"},
    {"op":"relationship","external_id":"AL_START","kind":"Allocate","source":"handle:UC_START","target":"ext:CTRL","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"AL_DRIVE","kind":"Allocate","source":"handle:UC_DRIVE","target":"ext:PT","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"AL_ACCEL","kind":"Allocate","source":"handle:UC_ACCEL","target":"ext:CTRL","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"AL_BRAKE","kind":"Allocate","source":"handle:UC_BRAKE","target":"ext:BRAKES","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"AL_CHARGE","kind":"Allocate","source":"handle:UC_CHARGE","target":"ext:ESS","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"AL_DIAG","kind":"Allocate","source":"handle:UC_DIAG","target":"ext:CTRL","owner":"ext:PKG_UC"},
    {"op":"relationship","external_id":"AL_VERIFY","kind":"Allocate","source":"handle:UC_VERIFY","target":"ext:VEH","owner":"ext:PKG_UC"}
  ]
}
''')
