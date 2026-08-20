(() => {
  'use strict';
  const host = document.createElement('div'); host.id = 'dialog-host'; document.body.appendChild(host);
  const notices = document.createElement('div'); notices.className = 'notification-host'; notices.setAttribute('aria-live','polite'); document.body.appendChild(notices);
  let active = null;
  function notify(message, level='info') { const item=document.createElement('div'); item.className=`notification ${level}`; item.textContent=message; notices.appendChild(item); setTimeout(()=>item.remove(),6000); }
  function cancelActive() { active?.cancel(); }
  function open({ title, description='', fields=[], candidates=[], searchable=false, confirmLabel='OK', destructive=false }) {
    cancelActive(); const invoking=document.activeElement;
    return new Promise((resolve) => {
      const overlay=document.createElement('div'); overlay.className='modal-overlay';
      overlay.innerHTML=`<section class="application-dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title"><header><h2 id="dialog-title"></h2></header><div class="dialog-body"><p class="dialog-description"></p><div class="dialog-fields"></div><div class="dialog-candidates"></div></div><footer><button type="button" data-cancel>Cancel</button><button type="button" data-confirm class="${destructive?'danger':'primary'}"></button></footer></section>`;
      overlay.querySelector('h2').textContent=title; overlay.querySelector('.dialog-description').textContent=description; overlay.querySelector('[data-confirm]').textContent=confirmLabel;
      const values={}; const fieldHost=overlay.querySelector('.dialog-fields');
      for(const field of fields){const label=document.createElement('label');label.textContent=field.label;const input=document.createElement(field.multiline?'textarea':'input');input.name=field.id;input.value=field.value||'';input.required=!!field.required;label.appendChild(input);fieldHost.appendChild(label);values[field.id]=input;}
      const candidateHost=overlay.querySelector('.dialog-candidates'); let selected=null;
      const render=(query='')=>{candidateHost.innerHTML='';for(const candidate of candidates.filter(c=>(c.label||'').toLowerCase().includes(query.toLowerCase()))){const button=document.createElement('button');button.type='button';button.className='dialog-candidate';button.textContent=candidate.label;button.onclick=()=>{selected=candidate.id;candidateHost.querySelectorAll('.selected').forEach(n=>n.classList.remove('selected'));button.classList.add('selected');};candidateHost.appendChild(button);}};
      if(searchable){const search=document.createElement('input');search.type='search';search.placeholder='Search';search.setAttribute('aria-label','Search candidates');search.oninput=()=>render(search.value);candidateHost.before(search);} render();
      const close=(result)=>{document.removeEventListener('keydown',key,true);overlay.remove();active=null;invoking?.focus?.();resolve(result);};
      const confirm=()=>{if([...Object.values(values)].some(input=>!input.reportValidity()))return;close({values:Object.fromEntries(Object.entries(values).map(([key,input])=>[key,input.value])),selectedId:selected});};
      const key=(event)=>{if(event.key==='Escape'){event.preventDefault();close(null);}if(event.key==='Enter'&&!event.shiftKey&&event.target.tagName!=='TEXTAREA'){event.preventDefault();confirm();}if(event.key==='Tab'){const focusable=[...overlay.querySelectorAll('button,input,textarea')];const first=focusable[0],last=focusable.at(-1);if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}}};
      overlay.querySelector('[data-cancel]').onclick=()=>close(null);overlay.querySelector('[data-confirm]').onclick=confirm;document.addEventListener('keydown',key,true);host.appendChild(overlay);(overlay.querySelector('input,textarea,.dialog-candidate,[data-confirm]'))?.focus();active={cancel:()=>close(null)};
    });
  }
  window.smpDialogs=Object.freeze({open,notify,cancelActive,choose:(options)=>open({...options,searchable:true}),edit:(options)=>open(options),confirm:async(options)=>!!(await open({...options,destructive:options.destructive,confirmLabel:options.confirmLabel||'Confirm'}))});
})();
