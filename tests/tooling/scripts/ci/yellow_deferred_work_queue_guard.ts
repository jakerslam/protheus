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
function ensureDir(rel:string):void{fs.mkdirSync(path.dirname(abs(rel)),{recursive:true})}
function main():void{
 const strict=boolFlag('strict',true);const policyPath=flag('policy','validation/conformance/contracts/yellow_deferred_work_queue_policy.json');const outJson=flag('out-json','core/local/artifacts/yellow_deferred_work_queue_guard_current.json');const outMd=flag('out-markdown','local/workspace/reports/YELLOW_DEFERRED_WORK_QUEUE_GUARD_CURRENT.md');
 const policy=json(policyPath);const report=read(policy.report_path);const todo=read('docs/workspace/todo/todo_archive_registry.json')+read('docs/workspace/todo/todo_registry.json');const violations:Violation[]=[];
 if(policy.type!=='yellow_deferred_work_queue_policy')violations.push({kind:'policy_type_invalid',path:policyPath,detail:'Wrong type'});
 if(policy.policy?.defer_until_red_section_clear!==true)violations.push({kind:'defer_policy_missing',path:policyPath,detail:'Must defer until red section is clear'});
 if(policy.policy?.forbid_silent_pull_forward!==true)violations.push({kind:'silent_pull_forward_allowed',path:policyPath,detail:'Silent pull-forward must be forbidden'});
 for(const id of policy.source_todos||[]){
  if(!todo.includes(String(id)))violations.push({kind:'source_todo_missing',path:'docs/workspace/todo',detail:String(id)});
  if(!report.includes(String(id)))violations.push({kind:'report_missing_todo',path:policy.report_path,detail:String(id)});
 }
 for(const token of ['Do not begin','Architecture and tooling deltas','Next SRS stream','Active priority']){
  if(!report.includes(token))violations.push({kind:'report_token_missing',path:policy.report_path,detail:token});
 }
 const payload={ok:violations.length===0,type:'yellow_deferred_work_queue_guard',generated_at:new Date().toISOString(),strict,source_todos:policy.source_todos,violations};
 ensureDir(outJson);fs.writeFileSync(abs(outJson),JSON.stringify(payload,null,2)+'\n');
 ensureDir(outMd);fs.writeFileSync(abs(outMd),`# Yellow Deferred Work Queue Guard\n\n- ok: ${payload.ok}\n- violations: ${violations.length}\n\n${violations.map(v=>`- ${v.kind}: ${v.detail}`).join('\n')||'- none'}\n`);
 console.log(JSON.stringify(payload,null,2));if(strict&&!payload.ok)process.exit(1);
}
main();
