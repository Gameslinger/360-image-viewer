# 360 Photo Viewer
This is a 360 image viewer written in rust. It allows you to view mirror ball projection pictures. You can view 180, 360, and dual 180 degree photos.

I want to thank [FrostKiwi]([FrostKiwi about mirrorball projections](https://github.com/FrostKiwi/Mirrorball) for his video on the mirrorball projection and webapp. I used his shaders as a starting point.

## Running
This photo-viewer supports two main options: the image view in degrees and a "twin" option. Passing an angle (e.g. 180, 360) specifies the view from the original image. Using the twin option (t or twin) allows you to view pictures with two 180 degree views. See the examples below for specific examples.

```bash
# Building
cargo b -r
mv target/release/360-photo-viewer .

# Usage
./360-photo-viewer [image] [view type]

# Examples
## 180 degree photo
./360-photo-viewer pictures/field.jpg 180
## 360 degree photo
## (I don't have a 360 picture yet)
./360-photo-viewer example.jpg 360
## Twin 180 degree photo
./360-photo-viewer pictures/bridge.jpg t
```

## Controls
Looking up/down/left/right can be done with the `wasd` or arrow keys. To zoom in and out, use the `q` and `e` keys. When using the twin mode, it may be necessary to scale up the two 180 degree images because the 360 camera may have some overlap. Use `r` and `f` to scale the source up or down. To exit, hit escape.

## Background
I recently stumbled on a video by [FrostKiwi about mirrorball projections](https://youtu.be/rJPKTCdk-WI). I was intrigued by how a reflective ball could capture the environment in a single picture. The ability to look around a 360 view provides a much more immersive experience than a normal picture.

I also have a [Samsung Gear 360](https://en.wikipedia.org/wiki/Samsung_Gear_360) camera which can take two 180 fisheye pictures making a 360 view. I had taken a lot of pictures but I couldn't find a way to view them. Armed with the understanding and code provided by FrostKiwi's video and example project, I was able to write my own photo viewer.
