import copy
import json
from pathlib import Path


root = Path(__file__).resolve().parent
transcript = json.loads((root / 'transcript.json').read_text())
requests = json.loads((root / 'requests.json').read_text())


def valid(events, model_requests, after):
    if after != "fixed-value":
        return False
    calls = [entry['received'] for entry in events
             if entry.get('received', {}).get('method') == 'item/tool/call']
    if len(calls) != 1 or len(model_requests) != 2:
        return False
    call = calls[0]
    if call['params']['tool'] != 'fixture_write' or call['params']['arguments'] != {}:
        return False
    replies = [entry['sent'] for entry in events
               if entry.get('sent', {}).get('id') == call['id'] and 'result' in entry['sent']]
    if len(replies) != 1 or replies[0]['result'] != {
            'contentItems': [{'type': 'inputText', 'text': 'permission denied'}], 'success': False}:
        return False
    outputs = [item for item in model_requests[1]['input'] if item.get('type') == 'function_call_output']
    return len(outputs) == 1 and outputs[0]['call_id'] == call['params']['callId'] and outputs[0]['output'] == 'permission denied'


wrong_output = copy.deepcopy(requests)
for item in wrong_output[1]['input']:
    if item.get('type') == 'function_call_output':
        item['output'] = 'wrong-value'
missing_call = [entry for entry in transcript if entry.get('received', {}).get('method') != 'item/tool/call']
checks = {'denial_round_trip': valid(transcript, requests, (root / "fixture-after.txt").read_text()),
          'unauthorized_write_rejected': not valid(transcript, requests, 'modified-without-permission'),
          'wrong_output_rejected': not valid(transcript, wrong_output, "fixed-value"),
          'missing_tool_call_rejected': not valid(missing_call, requests, "fixed-value")}
(root / 'validation.json').write_text(json.dumps(checks, indent=2))
print(json.dumps(checks, indent=2))
raise SystemExit(0 if all(checks.values()) else 1)
