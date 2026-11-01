#!/usr/bin/env python3
"""Merge only the two analytics routes into the current DEV Kong configuration."""
import json
import time
import urllib.error
from pathlib import Path
import urllib.parse
import urllib.request
import yaml

admin = 'http://127.0.0.1:8001'
with urllib.request.urlopen(admin + '/config', timeout=10) as response:
    original = json.load(response)['config']
config = yaml.safe_load(original)
runtime = Path(__file__).resolve().parents[1] / '.runtime'
runtime.mkdir(mode=0o700, exist_ok=True)
backup = runtime / 'kong.current.before.yml'
backup.write_text(original)
backup.chmod(0o600)
services = config.setdefault('services', [])
for name, port in [('corvus', 8086), ('reports', 8087)]:
    service_name = f'{name}-rust-dev'
    read_timeout=40000 if name=='corvus' else 60000
    existing = next((v for v in services if v.get('name') == service_name), None)
    if existing:
        existing.pop('url', None)
        existing.update(host=service_name, port=port, protocol='http', connect_timeout=2000, read_timeout=read_timeout)
        for route in existing.get('routes', []):
            if route.get('name') == f'{name}-rust-dev-route':
                route['hosts'] = ['dev.tordo.io', 'localhost', '127.0.0.1']
    else:
        services.append(dict(name=service_name, host=service_name, port=port, protocol='http', connect_timeout=2000, read_timeout=read_timeout, routes=[dict(name=f'{name}-rust-dev-route', paths=[f'/{name}'], hosts=['dev.tordo.io', 'localhost', '127.0.0.1'], strip_path=True, protocols=['http', 'https'])]))
payload = urllib.parse.urlencode({'config': yaml.safe_dump(config, sort_keys=False)}).encode()
request = urllib.request.Request(admin + '/config', data=payload, method='POST')
with urllib.request.urlopen(request, timeout=15) as response:
    assert response.status in (200, 201)
# Recreated containers can exchange IP addresses while Kong retains its DNS cache.
# Do not announce readiness until each route reaches the correct service.
for attempt in range(90):
    ready=True
    for name,expected in [('corvus','corvus'),('reports','tordo-reports')]:
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:8000/{name}/health',timeout=2) as response:
                health=json.load(response)
            ready=ready and health.get('service')==expected and health.get('status')=='ready'
        except (urllib.error.URLError,ValueError):ready=False
    if ready:break
    time.sleep(0.5)
else:raise SystemExit('Configuración aplicada, pero las rutas analytics DEV todavía no alcanzan el servicio esperado.')
print('Rutas DEV verificadas: /corvus/ y /reports/.')
