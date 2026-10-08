import os, signal, subprocess, sys, time
recipe = sys.argv[1]
p = subprocess.Popen(["just", recipe], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True, text=True)
time.sleep(1.0)
os.killpg(p.pid, signal.SIGINT)
out, _ = p.communicate(timeout=20)
print(out.rstrip()); print(f"returncode={p.returncode}")
