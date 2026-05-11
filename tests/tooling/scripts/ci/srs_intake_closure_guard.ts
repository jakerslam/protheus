#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
const ROOT = process.cwd();
type Violation = { kind: string; path: string; detail: string };
function flag(name: string, fallback = ''): string { const prefix=`--${name}=`; const direct=process.argv.slice(2).find(a=>a.startsWith(prefix)); if(direct) return direct.slice(prefix.length); const idx=process.argv.indexOf(`--${name}`); return idx>=0?process.argv[idx+1]:fallback; }
function boolFlag(name: string, fallback=false): boolean { const raw=flag(name, fallback?'1':'0'); return raw==='1'||raw==='true'; }
function abs(rel:string):string{return path.join(ROOT,rel)}
function read(rel:string):string{return fs.readFileSync(abs(rel),'utf8')}
function json(rel:string):any{return JSON.parse(read(rel))}
function ensureDir(rel:string):void{fs.mkdirSync(path.dirname(abs(rel)),{recursive:true})}
function main():void{
 const strict=boolFlag('strict',true);
 const manifestPath=flag('manifest','validation/conformance/contracts/srs_intake_closure_manifest_2026-05-09.json');
 const outJson=flag('out-json','core/local/artifacts/srs_intake_closure_guard_current.json');
 const outMd=flag('out-markdown','local/workspace/reports/SRS_INTAKE_CLOSURE_GUARD_CURRENT.md');
 const manifest=json(manifestPath); const report=read(manifest.report_path); const srs=read('docs/workspace/SRS.md'); const todo=read('docs/workspace/todo/todo_archive_registry.json')+read('docs/workspace/todo/todo_registry.json');
 const violations:Violation[]=[];
 if(manifest.type!=='srs_intake_closure_manifest') violations.push({kind:'manifest_type_invalid',path:manifestPath,detail:'Wrong type'});
 if(manifest.policy?.forbid_shell_or_orchestration_implementation_in_this_wave!==true) violations.push({kind:'policy_boundary_missing',path:manifestPath,detail:'Boundary policy must forbid shell/orchestration implementation in this wave'});
 const closures=Array.isArray(manifest.closures)?manifest.closures:[];
 if(closures.length<4) violations.push({kind:'closure_count_too_low',path:manifestPath,detail:'Expected four closure lanes'});
 for(const row of closures){
  const id=String(row.todo_id||'<missing>');
  for(const field of ['todo_id','srs_heading','owner_lane','evidence_path']) if(!String(row[field]||'').trim()) violations.push({kind:'closure_field_missing',path:manifestPath,detail:`${id} missing ${field}`});
  if(!Array.isArray(row.allowed_domains)||row.allowed_domains.length===0) violations.push({kind:'closure_domains_missing',path:manifestPath,detail:id});
  if(!Array.isArray(row.future_acceptance_criteria)||row.future_acceptance_criteria.length<3) violations.push({kind:'closure_acceptance_incomplete',path:manifestPath,detail:id});
  const headings=[row.srs_heading, ...(row.source_headings||[])].map(String);
  for(const heading of headings) if(!srs.includes(heading)) violations.push({kind:'srs_heading_missing',path:'docs/workspace/SRS.md',detail:heading});
  if(!todo.includes(id)) violations.push({kind:'todo_id_not_tracked',path:'docs/workspace/todo',detail:id});
  if(!report.includes(id) && !report.includes(String(row.srs_heading).replace(/ \(.+\)$/,''))) violations.push({kind:'closure_report_missing_lane',path:manifest.report_path,detail:id});
 }
 const payload={ok:violations.length===0,type:'srs_intake_closure_guard',generated_at:new Date().toISOString(),strict,closure_count:closures.length,violations};
 ensureDir(outJson); fs.writeFileSync(abs(outJson),JSON.stringify(payload,null,2)+'\n');
 ensureDir(outMd); fs.writeFileSync(abs(outMd),`# SRS Intake Closure Guard\n\n- ok: ${payload.ok}\n- closure_count: ${closures.length}\n- violations: ${violations.length}\n\n${violations.map(v=>`- ${v.kind}: ${v.detail}`).join('\n')||'- none'}\n`);
 console.log(JSON.stringify(payload,null,2)); if(strict&&!payload.ok) process.exit(1);
}
main();
