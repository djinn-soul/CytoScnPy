class A:
    _class_unique_x = 1
    def m(self):
        print(_class_unique_x) # Should NOT ref A._class_unique_x
