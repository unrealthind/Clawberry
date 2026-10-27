import argparse, base64, hashlib, shutil, uuid, collections, json, sqlite3, subprocess, tempfile, threading, time, urllib.request, urllib.error, socket
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
parser=argparse.ArgumentParser(description='Local-only assistant CLI/daemon smoke test')
parser.add_argument('--binary', default='target/debug/tinyclaw')
parser.add_argument('--local-ui', action='store_true', help='Exercise token-free direct-loopback UI mode')
args=parser.parse_args()
binary=str(Path(args.binary).resolve())
contexts=[]
requests=[]
responses=collections.deque()
class Model(BaseHTTPRequestHandler):
    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        contexts.append(body['messages'])
        requests.append(body)
        fixture=responses.popleft() if responses else {}
        if fixture.get('delay'): time.sleep(fixture['delay'])
        content=fixture.get('content','mock-response')
        data={'choices':[{'message':{'role':'assistant','content':content}}]}
        if not fixture.get('unknown_usage'): data['usage']={'prompt_tokens':2,'completion_tokens':2,'total_tokens':4}
        self.send_response(fixture.get('status',200)); self.send_header('Content-Type','application/json'); self.end_headers()
        try: self.wfile.write(json.dumps(data).encode())
        except (BrokenPipeError,ConnectionResetError): pass
    def log_message(self,*args): pass
model=ThreadingHTTPServer(('127.0.0.1',0),Model)
threading.Thread(target=model.serve_forever,daemon=True).start()
with tempfile.TemporaryDirectory(prefix='raaviclaw-core-smoke-') as directory:
    root=Path(directory)
    (root/'config.toml').write_text(f'default_provider = "custom:http://127.0.0.1:{model.server_port}"\ndefault_model = "fixture-model"\ndefault_temperature = 0.0\napi_key = "test-placeholder"\n[gateway]\npaired_tokens = ["fixture-owner-token"]\nwebhook_rate_limit_per_minute = 500\n')
    def profiles(budget=5000000):
        text=f'agent_daily_microusd = {budget}\ndeep_daily_microusd = 2000000\ntask_microusd = 2000000\n'
        for profile in ['fast','normal','deep']:
            text+=f'\n[profiles.{profile}]\nprovider = "custom:http://127.0.0.1:{model.server_port}"\nmodel = "fixture-{profile}"\ninput_microusd_per_million = 1000000\noutput_microusd_per_million = 1000000\nmax_output_tokens = 512\n'
        (root/'model-profiles.toml').write_text(text)
    profiles()
    prefix=[binary,'--config-dir',directory]
    def cli(*args):
        return subprocess.run(prefix+list(args),text=True,capture_output=True,check=True,timeout=40).stdout
    agents=json.loads(cli('agents'))
    assert [a['id'] for a in agents]==['janet','harley']
    cli('chat','janet','-m','business-938')
    cli('chat','harley','-m','personal-741')
    cli('chat','janet','-m','followup-938')
    assert 'business-938' not in json.dumps(contexts[1])
    assert 'personal-741' not in json.dumps(contexts[2])
    assert 'business-938' in json.dumps(contexts[2])
    with socket.socket() as s:
        s.bind(('127.0.0.1',0)); port=s.getsockname()[1]
    output=open(root/'daemon.log','w')
    process=subprocess.Popen(prefix+['daemon','--host','127.0.0.1','--port',str(port)]+(['--local-ui'] if args.local_ui else []),stdout=output,stderr=output)
    def request(path, payload=None, authenticated=False, extra_headers=None):
        headers={'Content-Type':'application/json'}
        if authenticated and not args.local_ui: headers['Authorization']='Bearer fixture-owner-token'
        if extra_headers: headers.update(extra_headers)
        req=urllib.request.Request(f'http://127.0.0.1:{port}'+path,data=None if payload is None else json.dumps(payload).encode(),headers=headers)
        try:
            with urllib.request.urlopen(req,timeout=35) as response: return response.status,json.load(response)
        except urllib.error.HTTPError as error: return error.code,json.load(error)
    try:
        for _ in range(100):
            if process.poll() is not None: raise RuntimeError((root/'daemon.log').read_text())
            try:
                if request('/health')[0]==200: break
            except urllib.error.URLError: time.sleep(.1)
        else: raise RuntimeError('Daemon did not start')
        assert request('/api/agents')[0]==(200 if args.local_ui else 401)
        if args.local_ui:
            assert request('/api/agents', extra_headers={'Origin':'http://127.0.0.1:5173'})[0]==200
            for headers in [{'Origin':'https://evil.example'}, {'Host':'evil.example'}, {'Sec-Fetch-Site':'cross-site'}, {'X-Forwarded-For':'127.0.0.1'}]:
                assert request('/api/agents', extra_headers=headers)[0]==401,headers
        assert request('/api/agents',authenticated=True)[0]==200
        status,settings=request('/api/agents/janet/settings',authenticated=True)
        assert status==200
        settings['value']['soul']='Persisted owner guidance marker'
        settings['value']['memory_notes']='Owned runtime note marker'
        settings['value']['default_profile']='fast'
        settings['value']['skills']=[{'id':'brief','name':'Evidence brief','description':'Owned text guidance','files':[{'path':'SKILL.md','content':'Use selected evidence. '*80+'Skill-tail-marker'}]}]
        status,saved=request('/api/agents/janet/settings',settings,True)
        assert status==200 and saved['revision']==1
        assert request('/api/agents/janet/settings',settings,True)[0]==409
        assert request('/api/agents/harley/settings',authenticated=True)[1]['value']['memory_notes']==''
        assert request('/api/agents/janet/operations',{'operation':'settings.save','api_key':'fixture-secret'},True)[0]==400
        for agent in ['janet','harley']:
            status,body=request('/api/chat/'+agent,{'message':'daemon-'+agent},True)
            assert status==200,(status,body)
            assert body['agent']==agent
        assert 'Persisted owner guidance marker' in json.dumps(contexts[-2])
        assert 'Owned runtime note marker' not in json.dumps(contexts[-1])
        assert 'Skill-tail-marker' in json.dumps(contexts[-2])
        assert 'Skill-tail-marker' not in json.dumps(contexts[-1])
        assert requests[-2]['model']=='fixture-fast'
        saved=request('/api/agents/janet/settings',authenticated=True)[1]
        saved['value']['default_profile']='normal'
        assert request('/api/agents/janet/settings',saved,True)[0]==200
        assert request('/api/chat/janet',{'message':'spoof','from':'harley'},True)[0]==400
        assert request('/api/chat/researcher',{'message':'unknown'},True)[0]==404
        assert 'personal-741' not in json.dumps(contexts[3])
        assert 'business-938' not in json.dumps(contexts[4])
        def op(operation, agent='janet', **fields):
            status,body=request('/api/agents/'+agent+'/operations',{'operation':operation,**fields},True)
            assert status==200,(operation,status,body)
            return body
        def denied(operation, agent='harley', **fields):
            assert request('/api/agents/'+agent+'/operations',{'operation':operation,**fields},True)[0]==400
        thread=op('thread.create',title='Permit focus')
        task=op('task.create',title='Review selected evidence',description='Distill recommendation',thread_id=thread['id'])
        denied('task.get',task_id=task['id'])
        denied('thread.read',thread_id=thread['id'])
        decision_id=str(uuid.uuid4())
        decision_fields={'thread_id':thread['id'],'id':decision_id,'content':'Preserve reviewed scope'}
        assert len(op('thread.decision.record',**decision_fields))==1
        assert len(op('thread.decision.record',**decision_fields))==1
        denied('thread.decisions',thread_id=thread['id'])
        denied('thread.decision.record',**decision_fields)
        denied('thread.decision.record',agent='janet',**{**decision_fields,'content':'Changed payload'})
        op('thread.summary',thread_id=thread['id'],summary='Compact permit summary')
        before=len(contexts)
        status,body=request('/api/chat/janet',{'message':'thread-marker-123','thread_id':thread['id'],'task_id':task['id'],'profile':'fast'},True)
        assert status==200 and body['session_id']==thread['id'],body
        assert 'business-938' not in json.dumps(contexts[before])
        assert requests[-1]['model']=='fixture-fast' and requests[-1]['max_tokens']==512
        request('/api/chat/janet',{'message':'Permit focus followup'},True)
        assert 'Compact permit summary' in json.dumps(contexts[-1])
        assert 'thread-marker-123' not in json.dumps(contexts[-1])
        op('file.attach',thread_id=thread['id'],task_id=task['id'],filename='source.txt',data_base64=base64.b64encode(b'selected file').decode())
        assert len(op('file.list',thread_id=thread['id']))==1
        denied('file.list',thread_id=thread['id'])
        auth={} if args.local_ui else {'Authorization':'Bearer fixture-owner-token'}
        upload=urllib.request.Request(f'http://127.0.0.1:{port}/api/agents/janet/threads/{thread["id"]}/uploads?filename=upload.txt',data=b'binary-upload-evidence',headers=auth,method='POST')
        with urllib.request.urlopen(upload,timeout=5) as response:
            assert response.status==201
            uploaded=json.load(response)
        file_request=urllib.request.Request(f'http://127.0.0.1:{port}/api/agents/janet/files/{uploaded["id"]}',headers=auth)
        with urllib.request.urlopen(file_request,timeout=5) as response:
            assert response.read()==b'binary-upload-evidence'
            assert response.headers['X-Content-Type-Options']=='nosniff'
        peer_request=urllib.request.Request(f'http://127.0.0.1:{port}/api/agents/harley/files/{uploaded["id"]}',headers=auth)
        try: urllib.request.urlopen(peer_request,timeout=5); raise AssertionError('Cross-owner download admitted')
        except urllib.error.HTTPError as error: assert error.code==404
        spec={'task_id':task['id'],'role':'document reviewer','objective':'Review scoped evidence',
              'context':[{'source_ref':'doc-1','source_type':'file','content':'worker-only-context-456'}],
              'profile':'fast','budget_microusd':100000,'timeout_secs':10,'allowed_tools':[],'allowed_credentials':[]}
        valid={'summary':'Evidence reviewed','findings':['A supported finding'],'confidence':0.8,
               'sources':[{'source_ref':'doc-1','source_type':'file'}],
               'artifacts':[{'filename':'review.md','content':'Distilled evidence'}],
               'memory_candidates':[{'content':'Project fact','category':'project','confidence':0.8,'source_ref':'doc-1','sensitivity':'ordinary'},
                                    {'content':'Sensitive fact','category':'financial','confidence':0.9,'source_ref':'doc-1','sensitivity':'sensitive'},
                                    {'content':'Credential-like fixture','category':'api token','confidence':0.9,'source_ref':'doc-1','sensitivity':'ordinary'}]}
        denied('agent.spawn',agent='janet',spec={**spec,'allowed_tools':['shell']})
        denied('agent.spawn',agent='janet',spec={**spec,'allowed_credentials':['private-api']})
        denied('agent.spawn',spec=spec)
        responses.append({'content':json.dumps(valid)})
        worker=op('agent.spawn',spec=spec)
        done=op('agent.wait',worker_id=worker['id'])
        assert done['status']=='completed' and not done['active'],done
        assert 'worker-only-context-456' in json.dumps(contexts[-1])
        assert 'business-938' not in json.dumps(contexts[-1]) and 'personal-741' not in json.dumps(contexts[-1])
        denied('agent.status',worker_id=worker['id'])
        denied('archive.read',worker_id=worker['id'],filename='summary.md')
        for filename in ['summary.md','transcript.jsonl','tool_calls.jsonl','shell.log','sources.json','costs.json','memory_candidates.json','review.md']:
            result=op('archive.read',worker_id=worker['id'],filename=filename)
            assert result['filename']==filename
        denied('archive.read',agent='janet',worker_id=worker['id'],filename='../config.toml')
        candidates=op('memory.candidates')
        ordinary=next(c for c in candidates if c['candidate']['category']=='project')
        sensitive=next(c for c in candidates if c['candidate']['sensitivity']=='sensitive')
        credential=next(c for c in candidates if c['candidate']['category']=='api token')
        denied('memory.promote',candidate_id=ordinary['id'])
        denied('memory.promote',agent='janet',candidate_id=sensitive['id'])
        denied('memory.promote',agent='janet',candidate_id=credential['id'])
        op('memory.promote',candidate_id=ordinary['id'])
        denied('memory.promote',agent='janet',candidate_id=ordinary['id'])
        op('memory.reject',candidate_id=sensitive['id'])
        memory=sqlite3.connect(agents[0]['memory_db'])
        row=memory.execute('SELECT content,confidence,source_ref FROM distilled_memories').fetchone()
        assert row[0]=='Project fact' and row[1]==0.8 and 'doc-1' in row[2],row
        memory.close()
        responses.append({'content':'invalid JSON'})
        failed=op('agent.spawn',spec=spec)
        assert op('agent.wait',worker_id=failed['id'])['status']=='failed'
        assert op('archive.read',worker_id=failed['id'],filename='transcript.jsonl')
        responses.append({'content':json.dumps(valid)})
        retry=op('agent.retry',worker_id=failed['id'])
        assert retry['id']!=failed['id']
        assert op('agent.wait',worker_id=retry['id'])['status']=='completed'
        responses.extend([{'status':500},{'content':json.dumps(valid)}])
        escalated=op('agent.spawn',spec={**spec,'retry_deep':True})
        assert op('agent.wait',worker_id=escalated['id'])['status']=='completed'
        assert requests[-1]['model']=='fixture-deep'
        responses.extend([{'content':'<internal_action>'+json.dumps({'operation':'task.create','title':'Agent-created task','description':'Prepared internally'})+'</internal_action>'},{'content':'Task prepared'}])
        assert request('/api/chat/janet',{'message':'Prepare a task'},True)[0]==200
        assert any(t['title']=='Agent-created task' for t in op('task.list'))
        responses.extend([{'content':'<internal_action>'+json.dumps({'operation':'reasoning.escalate','profile':'deep','reason':'Conflicting evidence'})+'</internal_action>'},{'content':'Deep recommendation'}])
        assert request('/api/chat/janet',{'message':'Investigate conflicting evidence'},True)[0]==200
        assert requests[-2]['model']=='fixture-normal' and requests[-1]['model']=='fixture-deep'
        responses.append({'content':'<internal_action>'+json.dumps({'operation':'shell','command':'touch forbidden'})+'</internal_action>'})
        assert request('/api/chat/janet',{'message':'Attempt ungranted execution'},True)[0]==502
        responses.append({'delay':2,'content':json.dumps(valid)})
        cancelled=op('agent.spawn',spec=spec)
        time.sleep(.2)
        op('agent.cancel',worker_id=cancelled['id'])
        assert op('agent.wait',worker_id=cancelled['id'])['status']=='cancelled'
        time.sleep(2)
        responses.append({'delay':2,'content':json.dumps(valid)})
        timed=op('agent.spawn',spec={**spec,'timeout_secs':1})
        assert op('agent.wait',worker_id=timed['id'])['status']=='failed'
        time.sleep(2)
        responses.append({'unknown_usage':True})
        assert request('/api/chat/janet',{'message':'usage-uncertain'},True)[0]==200
        usage=op('usage.list')
        assert any(u['status']=='usage_unknown' and u['cost_microusd']>0 for u in usage)
        assert any(u['worker_id']==worker['id'] and u['input_tokens']==2 and u['output_tokens']==2 and u['cost_microusd']==4 for u in usage)
        before=len(contexts)
        profiles(1)
        assert request('/api/chat/janet',{'message':'budget-refusal','profile':'deep'},True)[0]==502
        assert len(contexts)==before,'Budget refusal must precede provider request'
        profiles()
        # The authenticated SSE path uses the same budgeted conversation host.
        responses.extend([{'content':'<internal_action>'+json.dumps({'operation':'task.list'})+'</internal_action>'},{'content':'Stream complete 🌍'}])
        stream_req=urllib.request.Request(f'http://127.0.0.1:{port}/api/chat/janet/stream',data=json.dumps({'message':'stream-fixture'}).encode(),headers={'Content-Type':'application/json',**({} if args.local_ui else {'Authorization':'Bearer fixture-owner-token'})})
        with urllib.request.urlopen(stream_req,timeout=35) as response:
            events=[json.loads(line[5:]) for line in response.read().decode().splitlines() if line.startswith('data:')]
        assert events[-1]=={'type':'complete','response':'Stream complete 🌍'},events
        assert ''.join(e['text'] for e in events if e['type']=='delta')=='Stream complete 🌍',events
        assert '<internal_action>' not in json.dumps(events)
        assert op('chat.page')['items'][0]['content']=='Stream complete 🌍'
        assert not op('watch.settings')['enabled']
        assert op('shared.page')['items']==[]
        private=op('memory.page')['items'][0]
        shared=op('memory.share',id=private['id'],reviewed_hash=private['review_hash'])
        reviewed=op('shared.page')['items'][0]
        shared_fact={key:reviewed[key] for key in ['content','category','confidence','source_type','source_ref','sensitivity','expires_at']}
        shared_fact['content']='Explicitly reviewed shared fixture fact'
        denied('shared.save',id=shared['id'],reviewed_hash=reviewed['review_hash'],fact=shared_fact)
        op('shared.save',id=shared['id'],reviewed_hash=reviewed['review_hash'],fact=shared_fact)
        denied('shared.verify',agent='janet',id=shared['id'],reviewed_hash=reviewed['review_hash'])
        reviewed=op('shared.page')['items'][0]
        op('shared.stale',id=shared['id'],reviewed_hash=reviewed['review_hash'])
        reviewed=op('shared.page')['items'][0]
        assert reviewed['stale']
        op('shared.verify',id=shared['id'],reviewed_hash=reviewed['review_hash'])
        assert op('shared.page')['items'][0]['last_verified_at']
        assert op('memory.page')['items'][0]['content']==private['content']
        op('memory.unshare',id=shared['id'])
        exchange=op('cross.request',task_id=task['id'],recipient='harley',question='Which fixture hour?')
        assert op('cross.list',agent='harley')==[]
        denied('cross.respond',id=exchange['id'],response='10 AM')
        reviewed=op('cross.review_queue')[0]
        op('cross.review',id=exchange['id'],reviewed_hash=reviewed['question_hash'],decision='approve_question')
        op('cross.respond',agent='harley',id=exchange['id'],response='10 AM')
        assert op('cross.list')[0]['response'] is None
        reviewed=op('cross.review_queue')[0]
        op('cross.review',id=exchange['id'],reviewed_hash=reviewed['response_hash'],decision='approve_response')
        assert op('cross.list')[0]['response']=='10 AM'
        denied('cross.respond',id=exchange['id'],response='11 AM')
        assert op('shared.page')['items']==[]
        watched=op('task.create',title='Watched fixture deadline',description='Investigate supplied evidence only')
        op('task.metadata',task_id=watched['id'],priority='high',due_at='2020-01-01T00:00:00Z')
        responses.append({'content':json.dumps({'summary':'Fixture deadline needs User review','findings':['Owned deadline is overdue'],
            'confidence':0.9,'sources':[{'source_ref':watched['id'],'source_type':'task'}],
            'artifacts':[],'memory_candidates':[],'recommended_skills':[],'errors':[]})})
        schedule={**op('watch.settings'),'enabled':True,'quiet_start_utc':0,'quiet_end_utc':0,'daily_runs':1}
        assert op('notification.list')['items']==[]
        op('watch.configure',settings=schedule)
        for _ in range(95):
            notices=op('notification.list')['items']
            if notices: break
            time.sleep(1)
        else: raise AssertionError('Fixture watch did not investigate and notify')
        op('watch.configure',settings={**schedule,'enabled':False})
        assert len(notices)==1 and notices[0]['status']=='completed',notices
        notice=notices[0]
        assert notice['summary']=='Fixture deadline needs User review'
        assert any(item['source_ref']==watched['id'] for item in notice['evidence'])
        investigator=op('agent.status',worker_id=notice['worker_id'])
        assert investigator['status']=='completed' and not investigator['active']
        assert investigator['spec']['allowed_tools']==[] and investigator['spec']['allowed_credentials']==[]
        assert op('notification.list',agent='harley')['items']==[]
        denied('notification.read',id=notice['id'])
        op('notification.read',id=notice['id'])
        assert op('agent.recover')['recovered']==0
        assert not list(Path(agents[0]['workspace']).joinpath('workers').iterdir()),'Completed ephemeral workspaces must be removed'
    finally:
        process.terminate()
        try: process.wait(timeout=5)
        except subprocess.TimeoutExpired: process.kill(); process.wait()
        output.close()
    # Full stopped-tree backup/restore at the same administrator-owned path.
    # Only temporary fixture state is copied. No live credentials/state are used.
    with tempfile.TemporaryDirectory(prefix='raaviclaw-restore-smoke-') as backup_dir:
        snapshot=Path(backup_dir)/'snapshot'
        shutil.copytree(root,snapshot)
        def hashes(path):
            return {str(p.relative_to(path)):hashlib.sha256(p.read_bytes()).hexdigest() for p in path.rglob('*') if p.is_file()}
        expected=hashes(root)
        root.rename(Path(backup_dir)/'original')
        shutil.copytree(snapshot,root)
        assert hashes(root)==expected,'Stopped state restore changed file bytes'
        with sqlite3.connect(root/'workspace/assistants/data/system.db') as restored:
            assert restored.execute('PRAGMA integrity_check').fetchone()==('ok',)
            assert restored.execute('SELECT content FROM thread_decisions WHERE id=?',(decision_id,)).fetchone()==('Preserve reviewed scope',)
            assert restored.execute("SELECT count(*) FROM thread_decisions WHERE id=?",(decision_id,)).fetchone()==(1,)
            assert restored.execute('SELECT status FROM peer_exchanges WHERE id=?',(exchange['id'],)).fetchone()==('closed',)
            assert restored.execute('SELECT status,read FROM notifications WHERE id=?',(notice['id'],)).fetchone()==('completed',1)
            assert restored.execute('SELECT owner_agent FROM watch_runs WHERE id=?',(notice['run_id'],)).fetchone()==('janet',)
        assert Path(agents[0]['workspace']).joinpath('files',uploaded['id']).read_bytes()==b'binary-upload-evidence'
        assert Path(done['archive_path']).joinpath('artifacts/review.md').read_text()=='Distilled evidence'
        print('PASS: complete stopped fixture-state backup/restore, byte hashes, SQLite integrity, decisions, files and worker archives')
    cli('chat','harley','-m','after-daemon-restart')
    assert 'daemon-harley' in json.dumps(contexts[-1])
    assert 'business-938' not in json.dumps(contexts[-1])
assert 'test-placeholder' not in json.dumps(contexts)
model.shutdown()
print(('Local token-free mode: ' if args.local_ui else 'Paired mode: ')+'PASS: CLI/daemon isolation, tasks/threads/files, restricted workers/archives/memory, budget and failure handling, restart, authenticated SSE/internal-payload suppression, exact shared-memory/peer review and source-linked read-only watch recommendations')
