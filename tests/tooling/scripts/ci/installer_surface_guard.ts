#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
const ROOT=process.cwd();
type Violation={kind:string;path:string;detail:string};
type Advisory={kind:string;path:string;detail:string};
function flag(name:string,fallback=''):string{const prefix=`--${name}=`;const direct=process.argv.slice(2).find(a=>a.startsWith(prefix));if(direct)return direct.slice(prefix.length);const idx=process.argv.indexOf(`--${name}`);return idx>=0?process.argv[idx+1]:fallback;}
function boolFlag(name:string,fallback=false):boolean{const raw=flag(name,fallback?'1':'0');return raw==='1'||raw==='true'}
function abs(rel:string):string{return path.join(ROOT,rel)}
function read(rel:string):string{return fs.readFileSync(abs(rel),'utf8')}
function json(rel:string):any{return JSON.parse(read(rel))}
function exists(rel:string):boolean{return fs.existsSync(abs(rel))}
function size(rel:string):number{return fs.statSync(abs(rel)).size}
function ensureDir(rel:string):void{fs.mkdirSync(path.dirname(abs(rel)),{recursive:true})}
function commandExists(cmd:string):boolean{return spawnSync('sh',['-lc',`command -v ${cmd} >/dev/null 2>&1`],{cwd:ROOT}).status===0}
function main():void{
 const strict=boolFlag('strict',true);const policyPath=flag('policy','validation/conformance/contracts/installer_surface_policy.json');const outJson=flag('out-json','core/local/artifacts/installer_surface_guard_current.json');const outMd=flag('out-markdown','local/workspace/reports/INSTALLER_SURFACE_GUARD_CURRENT.md');
 const policy=json(policyPath);const violations:Violation[]=[];const advisories:Advisory[]=[];
 if(policy.type!=='installer_surface_policy')violations.push({kind:'policy_type_invalid',path:policyPath,detail:'Wrong type'});
 for(const rel of policy.entrypoints||[])if(!exists(rel))violations.push({kind:'installer_entrypoint_missing',path:rel,detail:'Missing installer entrypoint'});
 for(const rel of policy.module_contracts||[]){if(!exists(rel)){violations.push({kind:'installer_module_contract_missing',path:rel,detail:'Missing module contract'});continue;}const c=json(rel);if(!String(c.type||'').startsWith('installer_'))violations.push({kind:'installer_module_contract_type_invalid',path:rel,detail:String(c.type||'missing')});}
 if(exists('install.sh')){
  const bytes=size('install.sh'); if(bytes>policy.size_policy.max_install_sh_bytes)violations.push({kind:'install_sh_size_growth',path:'install.sh',detail:`${bytes} > ${policy.size_policy.max_install_sh_bytes}`});
  const r=spawnSync('bash',['-n','install.sh'],{cwd:ROOT,encoding:'utf8'}); if(r.status!==0)violations.push({kind:'install_sh_syntax_invalid',path:'install.sh',detail:(r.stderr||r.stdout||'bash -n failed').trim().slice(0,500)});
 }
 if(exists('install.ps1')){
  const bytes=size('install.ps1'); if(bytes>policy.size_policy.max_install_ps1_bytes)violations.push({kind:'install_ps1_size_growth',path:'install.ps1',detail:`${bytes} > ${policy.size_policy.max_install_ps1_bytes}`});
  if(commandExists('pwsh')){
   const ps=`$tokens=$null;$errors=$null;[System.Management.Automation.Language.Parser]::ParseFile('${abs('install.ps1').replace(/'/g,"''")}',[ref]$tokens,[ref]$errors)|Out-Null;if($errors.Count -gt 0){$errors|ForEach-Object{Write-Error $_.Message};exit 1}`;
   const r=spawnSync('pwsh',['-NoProfile','-Command',ps],{cwd:ROOT,encoding:'utf8'}); if(r.status!==0)violations.push({kind:'install_ps1_syntax_invalid',path:'install.ps1',detail:(r.stderr||r.stdout||'pwsh parser failed').trim().slice(0,800)});
  } else advisories.push({kind:'pwsh_unavailable',path:'install.ps1',detail:'PowerShell parser unavailable on this host; Windows parser check is advisory here.'});
 }
 for(const [rel,tokens] of Object.entries(policy.required_tokens||{})){const text=exists(rel)?read(rel):'';for(const token of tokens as string[])if(!text.includes(token))violations.push({kind:'installer_required_token_missing',path:rel,detail:token});}
 const payload={ok:violations.length===0,type:'installer_surface_guard',generated_at:new Date().toISOString(),strict,sizes:{install_sh:exists('install.sh')?size('install.sh'):null,install_ps1:exists('install.ps1')?size('install.ps1'):null},violations,advisories};
 ensureDir(outJson);fs.writeFileSync(abs(outJson),JSON.stringify(payload,null,2)+'\n');ensureDir(outMd);fs.writeFileSync(abs(outMd),`# Installer Surface Guard\n\n- ok: ${payload.ok}\n- install_sh_bytes: ${payload.sizes.install_sh}\n- install_ps1_bytes: ${payload.sizes.install_ps1}\n- violations: ${violations.length}\n- advisories: ${advisories.length}\n\n## Violations\n${violations.map(v=>`- ${v.kind}: ${v.path} ${v.detail}`).join('\n')||'- none'}\n\n## Advisories\n${advisories.map(v=>`- ${v.kind}: ${v.path} ${v.detail}`).join('\n')||'- none'}\n`);
 console.log(JSON.stringify(payload,null,2));if(strict&&!payload.ok)process.exit(1);
}
main();
