# 🔡 Text

To display Text in Matthias you will have to provide the following arguments:

* Position of the text: \[f32; 2]
* Size of the font: \[f32; 2]
* Text: String
* Color of the text: \[u8; 4]

<pre class="language-lua"><code class="lang-lua"><strong>--Position (x, y)
</strong><strong>position = {500., 500.}
</strong><strong>
</strong><strong>--Font size
</strong><strong>size = 30.
</strong><strong>
</strong><strong>--Text color (r, g, b, a(opacity))
</strong><strong>color = {255, 0, 0, 255}
</strong><strong>
</strong><strong>--The actual text we are going to display
</strong><strong>text = "Hello world!"
</strong><strong>
</strong><strong>--Draw text
</strong><strong>draw_text(position, size, text, color)
</strong></code></pre>
