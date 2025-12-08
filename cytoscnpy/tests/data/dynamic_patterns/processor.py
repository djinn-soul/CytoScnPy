from .models import User

def process():
    u = User()
    # Dynamic attribute check
    if hasattr(u, "save"):
        u.save()
