import random

width = 7
height = 36

half_x = width // 2
laser_ys = range(3, height - 2, 5)

spawnpoints = ";".join(f"{x},1:d" for x in range(1, width))
lasers = ";".join(f"0,{y}:r;{width-1},{y}:l" for y in laser_ys)
print(
    f"Name=RNG_belt Size={width},{height} Antenna={half_x},0 Reboot=0,1:r Checkpoints={half_x},{height-1} Spawnpoints={spawnpoints} Lasers={lasers}"
)
print(";".join("F:udlr" if x == half_x else "V" for x in range(width)))
print(";".join("F" for _ in range(width)))
for y in range(2, height - 1):
    for x in range(width):
        s = "B"
        s += random.choice("ffs")
        s += random.choice("uudllrr")
        if y in laser_ys:
            if x == 0:
                s = "F:l"
            elif x == width - 1:
                s = "F:r"
        print(s, end="\n" if x == width - 1 else ";")
print(";".join("F" if x == half_x else "V" for x in range(width)))
