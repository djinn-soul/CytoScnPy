from .models import User

def process(user):
    # Dynamic dispatch using hasattr check
    if hasattr(user, "save"):
        user.save()
