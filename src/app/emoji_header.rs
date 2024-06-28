pub struct AnimatedBlobs { name: String, bytes: &'static [u8] }
pub struct Blobs { name: String, bytes: &'static [u8] }
pub struct Icons { name: String, bytes: &'static [u8] }
pub struct Letters { name: String, bytes: &'static [u8] }
pub struct Numbers { name: String, bytes: &'static [u8] }
pub struct Turtles { name: String, bytes: &'static [u8] }
pub enum Emoji {
	AnimatedBlobs(AnimatedBlobs),
	Blobs(Blobs),
	Icons(Icons),
	Letters(Letters),
	Numbers(Numbers),
	Turtles(Turtles),

}