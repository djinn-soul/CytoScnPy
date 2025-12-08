class A:
    class_unique_x = 1
    def m(self):
        print(class_unique_x) # Should NOT ref A.class_unique_x
