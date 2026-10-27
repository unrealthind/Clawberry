#!/usr/bin/env python3
"""Verify actual configured model routes; never read or print provider secrets."""
import json
import urllib.request
import sys
base='http://127.0.0.1:8080'
def request(path, body=None):
    req=urllib.request.Request(base+path,data=None if body is None else json.dumps(body).encode(),headers={'Content-Type':'application/json'})
    with urllib.request.urlopen(req,timeout=150) as response:
        return json.load(response)
status=request('/api/runtime/status')
if '--status' in sys.argv:
    print(json.dumps({k:status[k] for k in ['credential_configured','credential_source','live_verified','verified_profiles']}))
    sys.exit(0 if status['credential_configured'] and status['live_verified'] else 1)
if not status['credential_configured']:
    sys.exit('Claude credential missing. Set ANTHROPIC_API_KEY in .env.local and restart scripts/dev-local.py. Never paste the token in chat.')
thread=request('/api/agents/janet/operations',{'operation':'thread.create','title':'Runtime connection verification'})
for profile in ['fast','normal','deep']:
    result=request('/api/chat/janet',{'message':'Local provider connection verification. Reply READY only.','thread_id':thread['id'],'profile':profile})
    if not result.get('response'):
        sys.exit('Provider response was not confirmed; inspect runtime activity before retrying.')
    print(profile+': provider request completed')
status=request('/api/runtime/status')
if not status['live_verified']:
    sys.exit('Current profiles were not all recorded as verified. Check configured providers/models.')
print('All configured routes verified against their actual provider. No external tool action was requested.')
