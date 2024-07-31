---
description: >-
  Please note that a rectangle is calculated from two points (The first and
  second point).
---

# ⬜ Rectangle

To display a Circle in Matthias you will have to provide the following arguments:

* First point of the Rectangle: \[f32; 2]
* Second point of the Rectangle: \[f32; 2]
* Is the Rectangle filled: boolean
* Color of the Rectangle : \[u8; 4]

```lua
--Position (x, y)
first_point = {500., 500.}

--Position (x, y)
second_point = {800., 800.}

--Color of the rectangle (r, g, b, a)
color = {255, 255, 255, 255}

--Is the rectangle filled
is_filled = true

--Draw rectangle
draw_rectangle(first_point, second_point, is_filled, color)
```
