#!/usr/bin/env python3
"""Bounded concurrent HTTP load probe for a running VSN Control Plane."""
from __future__ import annotations
import argparse, concurrent.futures, statistics, time, urllib.request, urllib.error

def one(url:str,timeout:float)->tuple[bool,float,int]:
    started=time.perf_counter()
    try:
        with urllib.request.urlopen(url,timeout=timeout) as response:
            response.read(64*1024)
            return 200 <= response.status < 500, (time.perf_counter()-started)*1000, response.status
    except urllib.error.HTTPError as e:
        return e.code < 500, (time.perf_counter()-started)*1000, e.code
    except Exception:
        return False,(time.perf_counter()-started)*1000,0

def percentile(values:list[float],p:float)->float:
    if not values:return 0.0
    v=sorted(values);idx=min(len(v)-1,max(0,round((len(v)-1)*p)));return v[idx]

def main()->int:
    ap=argparse.ArgumentParser();ap.add_argument('--url',default='http://127.0.0.1:8787/health');ap.add_argument('--requests',type=int,default=500);ap.add_argument('--concurrency',type=int,default=20);ap.add_argument('--timeout',type=float,default=5.0);ap.add_argument('--max-error-rate',type=float,default=0.01);ap.add_argument('--max-p95-ms',type=float,default=1000.0);args=ap.parse_args()
    requests=max(1,min(args.requests,100000));workers=max(1,min(args.concurrency,512));started=time.perf_counter()
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool: results=list(pool.map(lambda _:one(args.url,args.timeout),range(requests)))
    elapsed=time.perf_counter()-started;lat=[x[1] for x in results];ok=sum(1 for x in results if x[0]);errors=requests-ok;error_rate=errors/requests
    print(f'requests={requests} ok={ok} errors={errors} error_rate={error_rate:.4f} elapsed_s={elapsed:.3f} rps={requests/max(elapsed,1e-9):.1f}')
    print(f'latency_ms mean={statistics.mean(lat):.2f} p50={percentile(lat,.50):.2f} p95={percentile(lat,.95):.2f} p99={percentile(lat,.99):.2f} max={max(lat):.2f}')
    return 0 if error_rate<=args.max_error_rate and percentile(lat,.95)<=args.max_p95_ms else 2
if __name__=='__main__':raise SystemExit(main())
