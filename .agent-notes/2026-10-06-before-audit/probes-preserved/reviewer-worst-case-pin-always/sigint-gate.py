import os, signal, subprocess, sys, time
recipe = sys.argv[1]
for f in ("../stream.log", "../stream.status"):
    try: os.remove(f)
    except FileNotFoundError: pass
p = subprocess.Popen(["just", "gate-sim", recipe], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True, text=True)
time.sleep(1.0)
os.killpg(p.pid, signal.SIGINT)
out, _ = p.communicate(timeout=20)
print("gate output:", out.rstrip().replace("\n", " | ")); print(f"gate returncode={p.returncode}")
time.sleep(4.0)
print("stream log after 4s:", open("../stream.log").read().rstrip().replace("\n", " | "))
print("stream status:", open("../stream.status").read().strip() if os.path.exists("../stream.status") else "<none>")
