import subprocess
import threading
import queue
import os

class PythonExecutor:
    def __init__(self):
        self.process = None
        self.output_queue = queue.Queue()
        self.start_process()

    def start_process(self):
        if self.process:
            self.process.terminate()
        
        # Python インタラクティブモードで起動
        self.process = subprocess.Popen(
            ["python3", "-u", "-q", "-i"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=os.environ
        )

        # 出力を監視するスレッド
        threading.Thread(target=self._listen_stream, args=(self.process.stdout, "stdout"), daemon=True).start()
        threading.Thread(target=self._listen_stream, args=(self.process.stderr, "stderr"), daemon=True).start()

    def _listen_stream(self, stream, label):
        for line in iter(stream.readline, ""):
            self.output_queue.put((label, line))
        stream.close()

    def execute(self, code):
        # 実行完了を検知するためのマーカー
        marker = "---EXECUTION_DONE---"
        full_code = f"{code}\nprint('{marker}')\n"
        
        try:
            self.process.stdin.write(full_code)
            self.process.stdin.flush()
        except BrokenPipeError:
            self.start_process()
            self.process.stdin.write(full_code)
            self.process.stdin.flush()

        output = []
        while True:
            try:
                label, line = self.output_queue.get(timeout=5)
                if marker in line:
                    break
                output.append(f"[{label}] {line.strip()}")
            except queue.Empty:
                output.append("[system] Timeout waiting for output")
                break
        
        return "\n".join(output)

    def terminate(self):
        if self.process:
            self.process.terminate()
