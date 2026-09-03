// Systems-Modeler-Pro PR57 EV demo module: 52-parametrics-runtime-diagrams.groovy.
// Import modules in numeric order. Shared source namespace + stable External IDs make reapply idempotent.
// The semantic model remains rich; the presentation set is intentionally curated to 14 clean sales-demo views.
modelScript('''
{
  "source_namespace":"demo:ev-digital-engineering:v3",
  "operations": [
    {"op":"element","external_id":"CB_FORCE","kind":"ConstraintBlock","name":"TractiveForceEquation","owner":"ext:PKG_PARAM","documentation":"Executable analysis: F = m * a"},
    {"op":"element","external_id":"CPF_M","kind":"ConstraintParameter","name":"m","owner":"handle:CB_FORCE","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CPF_A","kind":"ConstraintParameter","name":"a","owner":"handle:CB_FORCE","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CPF_F","kind":"ConstraintParameter","name":"F","owner":"handle:CB_FORCE","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CP_FORCE","kind":"ConstraintProperty","name":"tractiveForceEquation","owner":"ext:VEH","type_ref":"handle:CB_FORCE","multiplicity":{"lower":1,"upper":1}},
    {"op":"parametric_metadata","element":"handle:CB_FORCE","constraint_expression":"F = m * a"},
    {"op":"binding","external_id":"BF_M","name":"massBinding","owner":"ext:VEH","source":{"role":"ext:V_MASS"},"target":{"role":"handle:CP_FORCE","parameter":"handle:CPF_M"}},
    {"op":"binding","external_id":"BF_A","name":"accelerationBinding","owner":"ext:VEH","source":{"role":"ext:V_ACCEL"},"target":{"role":"handle:CP_FORCE","parameter":"handle:CPF_A"}},
    {"op":"binding","external_id":"BF_F","name":"forceBinding","owner":"ext:VEH","source":{"role":"ext:V_FORCE"},"target":{"role":"handle:CP_FORCE","parameter":"handle:CPF_F"}},

    {"op":"element","external_id":"CB_POWER","kind":"ConstraintBlock","name":"ElectricalPowerEquation","owner":"ext:PKG_PARAM","documentation":"Executable analysis: P = V * I"},
    {"op":"element","external_id":"CPP_V","kind":"ConstraintParameter","name":"V","owner":"handle:CB_POWER","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CPP_I","kind":"ConstraintParameter","name":"I","owner":"handle:CB_POWER","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CPP_P","kind":"ConstraintParameter","name":"P","owner":"handle:CB_POWER","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CP_POWER","kind":"ConstraintProperty","name":"electricalPowerEquation","owner":"ext:PT","type_ref":"handle:CB_POWER","multiplicity":{"lower":1,"upper":1}},
    {"op":"parametric_metadata","element":"handle:CB_POWER","constraint_expression":"P = V * I"},
    {"op":"binding","external_id":"BP_V","name":"voltageBinding","owner":"ext:PT","source":{"role":"ext:V_VOLT"},"target":{"role":"handle:CP_POWER","parameter":"handle:CPP_V"}},
    {"op":"binding","external_id":"BP_I","name":"currentBinding","owner":"ext:PT","source":{"role":"ext:V_CURR"},"target":{"role":"handle:CP_POWER","parameter":"handle:CPP_I"}},
    {"op":"binding","external_id":"BP_P","name":"powerBinding","owner":"ext:PT","source":{"role":"ext:V_POWER"},"target":{"role":"handle:CP_POWER","parameter":"handle:CPP_P"}},

    {"op":"element","external_id":"CB_RANGE","kind":"ConstraintBlock","name":"RangeEquation","owner":"ext:PKG_PARAM","documentation":"Executable analysis: R = E / C"},
    {"op":"element","external_id":"CPR_E","kind":"ConstraintParameter","name":"E","owner":"handle:CB_RANGE","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CPR_C","kind":"ConstraintParameter","name":"C","owner":"handle:CB_RANGE","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CPR_R","kind":"ConstraintParameter","name":"R","owner":"handle:CB_RANGE","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CP_RANGE","kind":"ConstraintProperty","name":"rangeEquation","owner":"ext:VEH","type_ref":"handle:CB_RANGE","multiplicity":{"lower":1,"upper":1}},
    {"op":"parametric_metadata","element":"handle:CB_RANGE","constraint_expression":"R = E / C"},
    {"op":"binding","external_id":"BR_E","name":"energyBinding","owner":"ext:VEH","source":{"role":"ext:V_ENERGY"},"target":{"role":"handle:CP_RANGE","parameter":"handle:CPR_E"}},
    {"op":"binding","external_id":"BR_C","name":"consumptionBinding","owner":"ext:VEH","source":{"role":"ext:V_CONS"},"target":{"role":"handle:CP_RANGE","parameter":"handle:CPR_C"}},
    {"op":"binding","external_id":"BR_R","name":"rangeBinding","owner":"ext:VEH","source":{"role":"ext:V_RANGE"},"target":{"role":"handle:CP_RANGE","parameter":"handle:CPR_R"}},

    {"op":"element","external_id":"CB_THERM","kind":"ConstraintBlock","name":"ThermalMarginEquation","owner":"ext:PKG_PARAM","documentation":"Executable analysis: M = L - Q"},
    {"op":"element","external_id":"CPT_L","kind":"ConstraintParameter","name":"L","owner":"handle:CB_THERM","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CPT_Q","kind":"ConstraintParameter","name":"Q","owner":"handle:CB_THERM","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CPT_M","kind":"ConstraintParameter","name":"M","owner":"handle:CB_THERM","type_ref":"ext:REAL"},
    {"op":"element","external_id":"CP_THERM","kind":"ConstraintProperty","name":"thermalMarginEquation","owner":"ext:TMS","type_ref":"handle:CB_THERM","multiplicity":{"lower":1,"upper":1}},
    {"op":"parametric_metadata","element":"handle:CB_THERM","constraint_expression":"M = L - Q"},
    {"op":"binding","external_id":"BT_L","name":"coolingCapacityBinding","owner":"ext:TMS","source":{"role":"ext:V_COOL"},"target":{"role":"handle:CP_THERM","parameter":"handle:CPT_L"}},
    {"op":"binding","external_id":"BT_Q","name":"heatLoadBinding","owner":"ext:TMS","source":{"role":"ext:V_HEAT"},"target":{"role":"handle:CP_THERM","parameter":"handle:CPT_Q"}},
    {"op":"binding","external_id":"BT_M","name":"marginBinding","owner":"ext:TMS","source":{"role":"ext:V_MARGIN"},"target":{"role":"handle:CP_THERM","parameter":"handle:CPT_M"}},

    {"op":"relationship","external_id":"REF_FORCE_EQ","kind":"Refine","source":"handle:CB_FORCE","target":"ext:REQ_PERF_FORCE","owner":"ext:PKG_REQ"},
    {"op":"relationship","external_id":"REF_POWER_EQ","kind":"Refine","source":"handle:CB_POWER","target":"ext:REQ_PERF_POWER","owner":"ext:PKG_REQ"},
    {"op":"relationship","external_id":"REF_RANGE_EQ","kind":"Refine","source":"handle:CB_RANGE","target":"ext:REQ_PERF_RANGE","owner":"ext:PKG_REQ"},
    {"op":"relationship","external_id":"REF_THERM_EQ","kind":"Refine","source":"handle:CB_THERM","target":"ext:REQ_PERF_THERM","owner":"ext:PKG_REQ"},

    {"op":"element","external_id":"FLEET","kind":"Block","name":"DemoFleet","owner":"ext:PKG_CONFIG","documentation":"Structural runtime root with two independent ElectricVehicle occurrences for occurrence-specific execution and parametric values."},
    {"op":"element","external_id":"P_VEH_A","kind":"PartProperty","name":"vehicleA","owner":"handle:FLEET","type_ref":"ext:VEH","multiplicity":{"lower":1,"upper":1}},
    {"op":"element","external_id":"P_VEH_B","kind":"PartProperty","name":"vehicleB","owner":"handle:FLEET","type_ref":"ext:VEH","multiplicity":{"lower":1,"upper":1}},
    {"op":"element","external_id":"SUPPORT_SYS","kind":"Block","name":"FacilitySupportSystem","owner":"ext:PKG_CONFIG","documentation":"Deliberately unrelated branch used to explain out-of-scope change/verification traversal."},
    {"op":"element","external_id":"P_SUPPORT","kind":"PartProperty","name":"supportSystem","owner":"handle:FLEET","type_ref":"handle:SUPPORT_SYS","multiplicity":{"lower":1,"upper":1}},

    {"op":"relationship","external_id":"IMP_ARCH_COMMON","kind":"PackageImport","source":"ext:PKG_ARCH","target":"ext:PKG_COMMON","owner":"ext:PKG_ARCH","name":"reuse common types/interfaces","visibility":"Public"},
    {"op":"relationship","external_id":"IMP_BEHAV_ARCH","kind":"PackageImport","source":"ext:PKG_BEHAVIOR","target":"ext:PKG_ARCH","owner":"ext:PKG_BEHAVIOR","name":"behavior uses architecture","visibility":"Public"},
    {"op":"relationship","external_id":"IMP_PARAM_ARCH","kind":"PackageImport","source":"ext:PKG_PARAM","target":"ext:PKG_ARCH","owner":"ext:PKG_PARAM","name":"analysis constrains architecture","visibility":"Public"},
    {"op":"relationship","external_id":"IMP_REQ_ARCH","kind":"PackageImport","source":"ext:PKG_REQ","target":"ext:PKG_ARCH","owner":"ext:PKG_REQ","name":"requirements trace to architecture","visibility":"Public"}
  ],
  "diagrams": [
    {"external_id":"D_PKG","family":"Package","name":"EV Digital Engineering Project Overview","owner":"ext:PKG"},
    {"external_id":"D_REQ_SYS","family":"Requirement","name":"Core EV System Requirements","owner":"ext:PKG_REQ_SYS"},
    {"external_id":"D_REQ_VER","family":"Requirement","name":"Verification Requirements and Test Cases","owner":"ext:PKG_REQ_VER"},
    {"external_id":"D_BDD_VEH","family":"BDD","name":"Electric Vehicle System Breakdown","owner":"ext:PKG_ARCH_VEH"},
    {"external_id":"D_BDD_PT","family":"BDD","name":"Powertrain Component Breakdown","owner":"ext:PKG_ARCH_PT"},
    {"external_id":"D_IBD_VEH","family":"IBD","name":"Electric Vehicle Internal Interfaces","owner":"ext:PKG_ARCH_VEH","context":"ext:VEH"},
    {"external_id":"D_IBD_ESS","family":"IBD","name":"Energy Storage and Charging Internal View","owner":"ext:PKG_ARCH_ESS","context":"ext:ESS"},
    {"external_id":"D_UC","family":"Use Case","name":"EV Operational Use Cases","owner":"ext:PKG_UC","context":"ext:VEH"},
    {"external_id":"D_ACT_OPERATE","family":"Activity","name":"Operate Electric Vehicle","owner":"ext:PKG_ACT","semantic":"ext:ACT_OPERATE"},
    {"external_id":"D_ACT_START","family":"Activity","name":"Start Vehicle — Drill-Down","owner":"ext:PKG_ACT","semantic":"ext:ACT_START"},
    {"external_id":"D_SM_VEH","family":"State Machine","name":"Vehicle Operational Modes","owner":"ext:PKG_SM","semantic":"ext:SM_VEH"},
    {"external_id":"D_SEQ_START","family":"Sequence","name":"Vehicle Startup Sequence","owner":"ext:PKG_SEQ","semantic":"ext:SEQ_START"},
    {"external_id":"D_PAR_FORCE","family":"Parametric","name":"Tractive Force Analysis — Expected 4.5 kN","owner":"ext:PKG_PARAM","context":"ext:VEH"},
    {"external_id":"D_PAR_RANGE","family":"Parametric","name":"Driving Range Analysis — Expected 400 km","owner":"ext:PKG_PARAM","context":"ext:VEH"}
  ]
}
''')
