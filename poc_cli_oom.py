import subprocess
import time

massive_string = "hello " * 1_000_000 # 6MB

print("Testing CLI OOM boundaries...")
start = time.time()
proc = subprocess.Popen(
    ["cargo", "run", "--bin", "redberry", "--quiet", "--", "analyze", massive_string],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

stdout, stderr = proc.communicate()
print(f"Exit Code: {proc.returncode}")
if proc.returncode != 0:
    print("Vulnerability Unpatched! It panicked.")
    print(stderr[-500:])
else:
    print(f"Patched successfully! Executed in {time.time() - start:.2f}s")
