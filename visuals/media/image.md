# 🖼️ Image

To display an image in Matthias you will have to provide the following arguments:

* Position of the Image: \[f32; 2]
* Size of the Image: \[f32; 2]
* Path to the Image: String

<pre class="language-lua"><code class="lang-lua"><strong>--Position (x, y)
</strong><strong>position = {500., 500.}
</strong><strong>
</strong><strong>--Image Size (width, height)
</strong><strong>size = {100., 300.}
</strong><strong>
</strong><strong>--Draw image
</strong><strong>draw_image(position, size, "C:\\Path\\To\\Your\\Image.png")
</strong></code></pre>
