import os
import google.generativeai as genai
from dotenv import load_dotenv

load_dotenv()

class GeminiLLM:
    def __init__(self, model_name="gemini-1.5-flash"):
        api_key = os.getenv("GOOGLE_API_KEY")
        if not api_key:
            raise ValueError("GOOGLE_API_KEY not found in environment variables")
        
        genai.configure(api_key=api_key)
        self.model = genai.GenerativeModel(model_name)
        self.chat = self.model.start_chat(history=[])

    def ask(self, prompt):
        response = self.chat.send_message(prompt)
        return response.text
