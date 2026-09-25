const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const {test} = require('node:test');
function fixture({choice = 'existing', answers = [], rejectFirst = false} = {}) {
  const calls = [], dialogs = [];
  const state = {snapshot:{project:{elements:[{id:'whole',name:'Vehicle'},{id:'type',name:'Engine'},
    {id:'existing',kind:'PartProperty',owner_id:'whole',type_id:'type',name:'engine',multiplicity:'1'}]}}};
  const context = {state, console, document:{addEventListener() {}}, refresh:async()=>{},
    renderStatus() {}, runCommand:async(_, action)=>action(),
    requireInvoke:()=>async(command,args)=>{calls.push({command,args}); if(rejectFirst&&calls.length===1)throw new Error('Duplicate name'); return 'relationship';},
    window:{smpDialogs:{choose:async options=>{dialogs.push(options);return choice===null?null:{selectedId:choice};},
      edit:async options=>{dialogs.push(options);const values=answers.shift();return values?{values}:null;}, notify() {}}},
  };
  vm.createContext(context);
  for(const file of ['bdd-composition-authoring.js','undo-redo-ui.js'])vm.runInContext(fs.readFileSync(`${__dirname}/../apps/desktop/frontend/${file}`,'utf8'),context);
  return{context,calls,dialogs,state,run:()=>context.window.smpAuthorComposition('bdd','whole','type')};
}
test('reuse submits the selected stable usage and classifier IDs in one native transaction',async()=>{
  const ui=fixture();await ui.run();assert.equal(ui.calls.length,1);assert.equal(ui.calls[0].command,'author_part_composition');
  assert.equal(ui.calls[0].args.request.propertyId,'existing');assert.equal(ui.calls[0].args.request.typeId,'type');
  assert.equal(ui.calls[0].args.request.ownerId,'whole');assert.equal(ui.state.selectedRelationshipId,'relationship');
});
test('new usage submits explicit name/multiplicity and retains draft on rejection',async()=>{
  const ui=fixture({choice:'__new__',rejectFirst:true,answers:[{name:'engine',multiplicity:'4'},{name:'rearEngine',multiplicity:'4'}]});
  await ui.run();assert.equal(ui.calls.length,2);assert.equal(ui.calls[1].args.request.name,'rearEngine');
  assert.equal(ui.calls[1].args.request.multiplicity,'4');assert.equal(ui.calls[1].args.request.propertyId,null);
  assert.equal(ui.dialogs[2].fields[0].value,'engine');assert.match(ui.dialogs[2].description,/Duplicate name/);
});
test('cancel does not create a usage or checkpoint',async()=>{
  const ui=fixture({choice:null});await ui.run();assert.equal(ui.calls.length,0);
});
