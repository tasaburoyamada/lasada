import sys
from interpreter import SimpleInterpreter

def main():
    interpreter = SimpleInterpreter()
    print("My-Interpreter Started. Type 'exit' to quit.")
    
    try:
        while True:
            user_input = input("> ")
            if user_input.lower() in ["exit", "quit"]:
                break
            interpreter.chat(user_input)
    except KeyboardInterrupt:
        pass
    finally:
        interpreter.stop()
        print("\nGoodbye.")

if __name__ == "__main__":
    main()
