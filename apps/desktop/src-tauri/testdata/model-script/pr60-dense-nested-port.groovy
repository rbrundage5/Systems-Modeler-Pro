modelScript('''
{
  "source_namespace": "pr60:dense-nested-port",
  "operations": [
    {"op":"element","external_id":"PKG","kind":"Package","name":"Dense IBD Regression","owner":"$root"},
    {"op":"element","external_id":"IF","kind":"InterfaceBlock","name":"ControlInterface","owner":"handle:PKG"},
    {"op":"element","external_id":"SUBSYS","kind":"Block","name":"Subsystem","owner":"handle:PKG"},
    {"op":"element","external_id":"PA","kind":"ProxyPort","name":"PA","owner":"handle:SUBSYS","type_ref":"handle:IF"},
    {"op":"element","external_id":"PB","kind":"ProxyPort","name":"PB","owner":"handle:SUBSYS","type_ref":"handle:IF"},
    {"op":"element","external_id":"PC","kind":"ProxyPort","name":"PC","owner":"handle:SUBSYS","type_ref":"handle:IF"},
    {"op":"element","external_id":"PD","kind":"ProxyPort","name":"PD","owner":"handle:SUBSYS","type_ref":"handle:IF"},
    {"op":"element","external_id":"VEH","kind":"Block","name":"Vehicle","owner":"handle:PKG"},
    {"op":"element","external_id":"P1","kind":"PartProperty","name":"part1","owner":"handle:VEH","type_ref":"handle:SUBSYS"},
    {"op":"element","external_id":"P2","kind":"PartProperty","name":"part2","owner":"handle:VEH","type_ref":"handle:SUBSYS"},
    {"op":"element","external_id":"P3","kind":"PartProperty","name":"part3","owner":"handle:VEH","type_ref":"handle:SUBSYS"},
    {"op":"element","external_id":"P4","kind":"PartProperty","name":"part4","owner":"handle:VEH","type_ref":"handle:SUBSYS"},
    {"op":"element","external_id":"P5","kind":"PartProperty","name":"part5","owner":"handle:VEH","type_ref":"handle:SUBSYS"},
    {"op":"element","external_id":"P6","kind":"PartProperty","name":"part6","owner":"handle:VEH","type_ref":"handle:SUBSYS"},
    {"op":"element","external_id":"P7","kind":"PartProperty","name":"part7","owner":"handle:VEH","type_ref":"handle:SUBSYS"},
    {"op":"element","external_id":"P8","kind":"PartProperty","name":"part8","owner":"handle:VEH","type_ref":"handle:SUBSYS"},
    {"op":"element","external_id":"P9","kind":"PartProperty","name":"part9","owner":"handle:VEH","type_ref":"handle:SUBSYS"},
    {"op":"connector","external_id":"E1","context":"handle:VEH","kind":"Assembly","source_path":["handle:P5","handle:PA"],"target_path":["handle:P1","handle:PA"],"name":"E1"},
    {"op":"connector","external_id":"E2","context":"handle:VEH","kind":"Assembly","source_path":["handle:P5","handle:PA"],"target_path":["handle:P2","handle:PA"],"name":"E2"},
    {"op":"connector","external_id":"E3","context":"handle:VEH","kind":"Assembly","source_path":["handle:P5","handle:PA"],"target_path":["handle:P3","handle:PA"],"name":"E3"},
    {"op":"connector","external_id":"E4","context":"handle:VEH","kind":"Assembly","source_path":["handle:P5","handle:PA"],"target_path":["handle:P4","handle:PA"],"name":"E4"},
    {"op":"connector","external_id":"E5","context":"handle:VEH","kind":"Assembly","source_path":["handle:P5","handle:PA"],"target_path":["handle:P6","handle:PA"],"name":"E5"},
    {"op":"connector","external_id":"E6","context":"handle:VEH","kind":"Assembly","source_path":["handle:P5","handle:PA"],"target_path":["handle:P7","handle:PA"],"name":"E6"},
    {"op":"connector","external_id":"E7","context":"handle:VEH","kind":"Assembly","source_path":["handle:P5","handle:PA"],"target_path":["handle:P8","handle:PA"],"name":"E7"},
    {"op":"connector","external_id":"E8","context":"handle:VEH","kind":"Assembly","source_path":["handle:P5","handle:PA"],"target_path":["handle:P9","handle:PA"],"name":"E8"},
    {"op":"connector","external_id":"E9","context":"handle:VEH","kind":"Assembly","source_path":["handle:P1","handle:PB"],"target_path":["handle:P9","handle:PB"],"name":"E9"},
    {"op":"connector","external_id":"E10","context":"handle:VEH","kind":"Assembly","source_path":["handle:P3","handle:PC"],"target_path":["handle:P7","handle:PC"],"name":"E10"},
    {"op":"connector","external_id":"E11","context":"handle:VEH","kind":"Assembly","source_path":["handle:P2","handle:PD"],"target_path":["handle:P8","handle:PD"],"name":"E11"}
  ],
  "diagrams": [
    {"external_id":"D_IBD","family":"IBD","name":"Dense Nested Port IBD","owner":"handle:PKG","context":"handle:VEH","populate":true,"clean_layout":true,"route":true}
  ]
}
''')
