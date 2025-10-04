// GIF Preview Canvas
export class GifPreview {
  constructor(canvas, frames) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
    this.frames = frames;
    this.startFrame = 0;
    this.endFrame = frames.length;
    this.currentFrame = 0;
    this.animationFrame = null;
  }
  
  setRange(start, end) {
    this.startFrame = start;
    this.endFrame = end;
    this.currentFrame = start;
  }
  
  render() {
    if (this.frames.length === 0) return;
    
    // For now, just show first frame
    // In full implementation, this would animate through frames
    const firstFrame = this.frames[0];
    
    if (firstFrame instanceof ImageData) {
      this.canvas.width = firstFrame.width;
      this.canvas.height = firstFrame.height;
      this.ctx.putImageData(firstFrame, 0, 0);
    } else if (firstFrame instanceof HTMLCanvasElement) {
      this.canvas.width = firstFrame.width;
      this.canvas.height = firstFrame.height;
      this.ctx.drawImage(firstFrame, 0, 0);
    }
  }
  
  startAnimation(fps = 15) {
    const frameTime = 1000 / fps;
    let lastTime = 0;
    
    const animate = (time) => {
      if (time - lastTime >= frameTime) {
        this.currentFrame++;
        if (this.currentFrame >= this.endFrame) {
          this.currentFrame = this.startFrame;
        }
        
        const frame = this.frames[this.currentFrame];
        if (frame instanceof ImageData) {
          this.ctx.putImageData(frame, 0, 0);
        } else if (frame instanceof HTMLCanvasElement) {
          this.ctx.drawImage(frame, 0, 0);
        }
        
        lastTime = time;
      }
      
      this.animationFrame = requestAnimationFrame(animate);
    };
    
    this.animationFrame = requestAnimationFrame(animate);
  }
  
  stopAnimation() {
    if (this.animationFrame) {
      cancelAnimationFrame(this.animationFrame);
      this.animationFrame = null;
    }
  }
}