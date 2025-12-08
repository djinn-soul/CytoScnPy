from .handlers import Handler

def serve(action):
    h = Handler()
    # Dynamic dispatch using hasattr check
    # This should mark 'handle_login' and 'handle_logout' as used
    if action == "login":
        if hasattr(h, "handle_login"):
            h.handle_login()
    elif action == "logout":
        if hasattr(h, "handle_logout"):
            h.handle_logout()
