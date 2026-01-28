from enum_def import Status

def check_status(s):
    if s == Status.ACTIVE:
        print("Active")
    elif s == Status.PENDING:
        print("Pending")
