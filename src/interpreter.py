import re
from .llm import GeminiLLM
from .executor import PythonExecutor

class SimpleInterpreter:
    def __init__(self):
        self.llm = GeminiLLM()
        self.executor = PythonExecutor()
        self.system_prompt = """
あなたはユーザーのコンピュータ上でコードを実行できるAIアシスタントです。
指示に対してPythonコードを作成し、実行して結果を確認してください。
コードは必ず以下の形式で記述してください：
```python
(ここにコード)
```
実行結果は自動的にあなたにフィードバックされます。タスクが完了するまでこれを繰り返してください。
"""
        # 初期プロンプトの設定
        self.llm.chat.history = [
            {"role": "user", "parts": [self.system_prompt]},
            {"role": "model", "parts": ["了解しました。指示をどうぞ。"]}
        ]

    def _extract_code(self, text):
        pattern = r"```python\n(.*?)```"
        matches = re.findall(pattern, text, re.DOTALL)
        return matches

    def chat(self, user_input):
        print(f"\n[User] {user_input}")
        current_input = user_input

        while True:
            response_text = self.llm.ask(current_input)
            print(f"\n[AI] {response_text}")

            codes = self._extract_code(response_text)
            if not codes:
                break
            
            for code in codes:
                print(f"\n[Executing...]\n{code}")
                result = self.executor.execute(code)
                print(f"\n[Result]\n{result}")
                current_input = f"Execution Result:\n{result}\n\nProceed based on this result."
            
            # コードが実行された後はループを継続してLLMに結果を評価させる
            # ただし、同じコードを何度も実行しないように制御が必要（MVPでは一旦単純ループ）
            
    def stop(self):
        self.executor.terminate()
