#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
const ROOT=process.cwd();
type Violation={kind:string;path:string;detail:string};
function flag(name:string,fallback=''):string{const prefix=`--${name}=`;const direct=process.argv.slice(2).find(a=>a.startsWith(prefix));if(direct)return direct.slice(prefix.length);const idx=process.argv.indexOf(`--${name}`);return idx>=0?process.argv[idx+1]:fallback;}
function boolFlag(name:string,fallback=false):boolean{const raw=flag(name,fallback?'1':'0');return raw==='1'||raw==='true'}
function abs(rel:string):string{return path.join(ROOT,rel)}
function json(rel:string):any{return JSON.parse(fs.readFileSync(abs(rel),'utf8'))}
function ensureDir(rel:string):void{fs.mkdirSync(path.dirname(abs(rel)),{recursive:true})}
function main():void{
 const strict=boolFlag('strict',true); const policyPath=flag('policy','validation/conformance/contracts/rust_hygiene_baseline_policy.json'); const outJson=flag('out-json','core/local/artifacts/rust_hygiene_baseline_guard_current.json'); const outMd=flag('out-markdown','local/workspace/reports/RUST_HYGIENE_BASELINE_GUARD_CURRENT.md');
 const policy=json(policyPath); const cls=json(policy.classification_artifact); const base=json(policy.deadcode_baseline_artifact); const inv=json(policy.source_artifacts[0]); const refs=json(policy.source_artifacts[1]); const violations:Violation[]=[];
 if(policy.type!=='rust_hygiene_baseline_policy') violations.push({kind:'policy_type_invalid',path:policyPath,detail:'Wrong type'});
 if(cls.type!=='rust_combined_split_debt_classification') violations.push({kind:'classification_type_invalid',path:policy.classification_artifact,detail:'Wrong type'});
 if(cls.artifact_count!==inv.artifact_count||cls.artifact_count!==refs.artifact_count) violations.push({kind:'classification_count_mismatch',path:policy.classification_artifact,detail:`classification=${cls.artifact_count} inventory=${inv.artifact_count} refs=${refs.artifact_count}`});
 for(const key of ['live_split_debt','deletion_candidate_review']) if(!cls.summary?.[key]) violations.push({kind:'classification_required_class_missing',path:policy.classification_artifact,detail:key});
 if(base.type!=='rust_deadcode_warning_baseline') violations.push({kind:'deadcode_baseline_type_invalid',path:policy.deadcode_baseline_artifact,detail:'Wrong type'});
 if(base.complete===false && !String(base.compile_blocker?.detail||base.compile_blocker?.path_hint||'').includes('unclosed delimiter')) violations.push({kind:'partial_baseline_without_compile_blocker',path:policy.deadcode_baseline_artifact,detail:'Partial baseline must record the current compile blocker'});
 if(!Number.isInteger(base.warning_count_seen_before_blocker)||base.warning_count_seen_before_blocker<1) violations.push({kind:'deadcode_warning_samples_missing',path:policy.deadcode_baseline_artifact,detail:'Expected warnings before blocker'});
 const payload={ok:violations.length===0,type:'rust_hygiene_baseline_guard',generated_at:new Date().toISOString(),strict,classification_count:cls.artifact_count,classification_summary:cls.summary,deadcode_baseline_complete:base.complete,warning_count_seen_before_blocker:base.warning_count_seen_before_blocker,violations};
 ensureDir(outJson);fs.writeFileSync(abs(outJson),JSON.stringify(payload,null,2)+'\n'); ensureDir(outMd);fs.writeFileSync(abs(outMd),`# Rust Hygiene Baseline Guard\n\n- ok: ${payload.ok}\n- classification_count: ${cls.artifact_count}\n- deadcode_baseline_complete: ${base.complete}\n- violations: ${violations.length}\n\n${violations.map(v=>`- ${v.kind}: ${v.detail}`).join('\n')||'- none'}\n`);
 console.log(JSON.stringify(payload,null,2)); if(strict&&!payload.ok)process.exit(1);
}
main();
