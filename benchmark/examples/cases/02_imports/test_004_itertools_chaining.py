import itertools

class FalseResponseBase:
    def false_negative_series_generator(self):
        return itertools.cycle([False, False, True])

class Controller:
    def __init__(self):
        self.false_responses = FalseResponseBase()

    def check(self):
        value = next(self.false_responses.false_negative_series_generator())
        print(f"False negative check: {value}")
        return value

if __name__ == "__main__":
    ctrl = Controller()
    ctrl.check()
