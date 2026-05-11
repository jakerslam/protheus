#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
const ROOT=process.cwd();
type Violation={kind:string;path:string;detail:string};
function flag(name:string,fallback=''):string{const prefix=`--${name}=`;const direct=process.argv.slice(2).find(a=>a.startsWith(prefix));if(direct)return direct.slice(prefix.length);const idx=process.argv.indexOf(`--${name}`);return idx>=0?process.argv[idx+1]:fallback;}
function boolFlag(name:string,fallback=false):boolean{const raw=flag(name,fallback?'1':'0');return raw==='1'||raw==='true'}
function abs(rel:string):string{return path.join(ROOT,rel)}
function read(rel:string):string{return fs.readFileSync(abs(rel),'utf8')}
function json(rel:string):any{return JSON.parse(read(rel))}
function exists(rel:string):boolean{return fs.existsSync(abs(rel))}
function ensureDir(rel:string):void{fs.mkdirSync(path.dirname(abs(rel)),{recursive:true})}
function main():void{
 const strict=boolFlag('strict',true);const policyPath=flag('policy','validation/conformance/contracts/blocked_hygiene_mutation_policy.json');const outJson=flag('out-json','core/local/artifacts/blocked_hygiene_mutation_guard_current.json');const outMd=flag('out-markdown','local/workspace/reports/BLOCKED_HYGIENE_MUTATION_GUARD_CURRENT.md');
 const policy=json(policyPath);const todo=json('docs/workspace/todo/todo_registry.json');const violations:Violation[]=[];
 if(policy.type!=='blocked_hygiene_mutation_policy')violations.push({kind:'policy_type_invalid',path:policyPath,detail:'Wrong type'});
 if(policy.policy?.forbid_mutating_cleanup_while_compile_blocked!==true)violations.push({kind:'mutation_policy_missing',path:policyPath,detail:'Must forbid cleanup while compile blocked'});
 for(const ref of policy.evidence_refs||[])if(!exists(String(ref)))violations.push({kind:'evidence_ref_missing',path:policyPath,detail:String(ref)});
 const active=new Map((todo.items||[]).map((i:any)=>[String(i.id),i]));
 for(const id of policy.blocked_todos||[]){
  const item=active.get(String(id));
  if(!item)continue;
  if(!String(item.summary||'').includes('blocked'))violations.push({kind:'blocked_todo_summary_missing_blocked',path:'docs/workspace/todo/todo_registry.json',detail:String(id)});
  if(item.section!=='yellow')violations.push({kind:'blocked_todo_wrong_section',path:'docs/workspace/todo/todo_registry.json',detail:`${id} should remain yellow until blocker clears`});
 }
 const baseline=json('validation/reports/rust_deadcode_warning_baseline_2026-05-09.json');
 if(baseline.complete!==false||baseline.compile_blocker?.kind!=='unclosed_delimiter')violations.push({kind:'compile_blocker_state_unexpected',path:'validation/reports/rust_deadcode_warning_baseline_2026-05-09.json',detail:'Expected partial baseline blocked by unclosed delimiter'});
 const payload={ok:violations.length===0,type:'blocked_hygiene_mutation_guard',generated_at:new Date().toISOString(),strict,blocked_todos:policy.blocked_todos,violations};
 ensureDir(outJson);fs.writeFileSync(abs(outJson),JSON.stringify(payload,null,2)+'\n');
 ensureDir(outMd);fs.writeFileSync(abs(outMd),`# Blocked Hygiene Mutation Guard\n\n- ok: ${payload.ok}\n- violations: ${violations.length}\n\n${violations.map(v=>`- ${v.kind}: ${v.detail}`).join('\n')||'- none'}\n`);
 console.log(JSON.stringify(payload,null,2));if(strict&&!payload.ok)process.exit(1);
}
main();
