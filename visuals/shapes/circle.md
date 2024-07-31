# ⚪ Circle

To display a Circle in Matthias you will have to provide the following arguments:

* Position of the Circle: \[f32; 2]
* Radius of the Circle: f32
* Is the Circle filled: boolean
* Color of the Circle: \[u8; 4]

```lua
--Position (x, y)
position = {500., 500.}

--circle Size (r)
radius = 20.

--Is the circle filled
is_filled = true

--Color of the circle (r, g, b, a)
color = {255, 255, 255, 255}

--Draw circle
draw_circle(position, radius, is_filled, color)
```
