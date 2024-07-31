# 📏 Line

To display a Circle in Matthias you will have to provide the following arguments:

* Starting position of the Line: \[f32; 2]
* End position of the Line: \[f32; 2]
* Color of the Line: \[u8; 4]

```lua
--Position (x, y)
start_position = {500., 500.}

--Position (x, y)
end_position = {800., 800.}

--Color of the circle (r, g, b, a)
color = {255, 255, 255, 255}

--Draw line
draw_line(position, radius, is_filled, color)
```
