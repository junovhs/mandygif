import { ScreenSelector } from './screen-selector.js';
import { Timeline } from './timeline.js';
import { GifPreview } from './gif-preview.js';
import { toast } from './utils.js';

const $ = (s) => document.querySelector(s);

class Recorder {
  constructor() {
    this.state = 'welcome';
    this.recordedFrames = [];
    this.recordingStartTime = 0;
    this.recordingInterval = null;
    this.stream = null;
    this.captureCanvas = null;
    this.captureCtx = null;
    this.captureInterval = null;
    this.bounds = null;
    this.captureFrameRate = 30;
    
    this.screenSelector = new ScreenSelector();
    this.timeline = null;
    this.gifPreview = null;
    
    this.initUI();
    this.setupRecordingHandler();
  }
  
  initUI() {
    $('#btn-new-recording').onclick = () => this.startNewRecording();
    $('#btn-stop-recording').onclick = () => this.stopRecording();
    $('#btn-export').onclick = () => this.exportFile();
    
    $('#format-select').onchange = () => this.updateEstimate();
    $('#fps-select').onchange = () => this.updateEstimate();
    $('#quality-select').onchange = () => this.updateEstimate();
  }
  
  setupRecordingHandler() {
    window.electronAPI.onRecordingSourceReady(async ({ sourceId, bounds }) => {
      await this.startCapture(sourceId, bounds);
    });
    
    window.electronAPI.onCaptureBoundsReady(async (bounds) => {
      const result = await window.electronAPI.startRecording({
        sourceId: 'screen:0:0',
        bounds: bounds
      });
      
      if (!result.success) {
        toast('Failed to start recording: ' + result.error);
      }
    });
    
    window.electronAPI.onCaptureCancelled(() => {
      // Do nothing, just cancelled
    });
  }
  
  async startNewRecording() {
    const overlayResult = await window.electronAPI.showCaptureOverlay();
    if (!overlayResult.success) {
      toast('Failed to show overlay');
    }
  }
  
  async startCapture(sourceId, bounds) {
    try {
      this.stream = await navigator.mediaDevices.getUserMedia({
        audio: false,
        video: {
          mandatory: {
            chromeMediaSource: 'desktop',
            chromeMediaSourceId: sourceId,
            minWidth: 1280,
            maxWidth: 4096,
            minHeight: 720,
            maxHeight: 2160
          }
        }
      });
      
      this.captureCanvas = document.createElement('canvas');
      this.captureCanvas.width = bounds.width;
      this.captureCanvas.height = bounds.height;
      this.captureCtx = this.captureCanvas.getContext('2d', { 
        willReadFrequently: true 
      });
      
      const video = document.createElement('video');
      video.srcObject = this.stream;
      video.play();
      
      await new Promise(resolve => {
        video.onloadedmetadata = resolve;
      });
      
      this.recordedFrames = [];
      this.bounds = bounds;
      
      const frameInterval = 1000 / this.captureFrameRate;
      
      this.captureInterval = setInterval(() => {
        this.captureCtx.drawImage(
          video,
          bounds.x, bounds.y, bounds.width, bounds.height,
          0, 0, bounds.width, bounds.height
        );
        
        const frameData = this.captureCtx.getImageData(0, 0, bounds.width, bounds.height);
        this.recordedFrames.push(frameData);
      }, frameInterval);
      
      this.setState('recording');
      this.startRecordingTimer();
      
    } catch (err) {
      console.error('Capture error:', err);
      toast('Failed to capture screen: ' + err.message);
    }
  }
  
  startRecordingTimer() {
    this.recordingStartTime = Date.now();
    this.recordingInterval = setInterval(() => {
      const elapsed = Math.floor((Date.now() - this.recordingStartTime) / 1000);
      const minutes = Math.floor(elapsed / 60);
      const seconds = elapsed % 60;
      $('.rec-time').textContent = `${minutes}:${seconds.toString().padStart(2, '0')}`;
    }, 1000);
  }
  
  async stopRecording() {
    if (this.captureInterval) {
      clearInterval(this.captureInterval);
      this.captureInterval = null;
    }
    
    if (this.recordingInterval) {
      clearInterval(this.recordingInterval);
      this.recordingInterval = null;
    }
    
    if (this.stream) {
      this.stream.getTracks().forEach(track => track.stop());
      this.stream = null;
    }
    
    if (this.recordedFrames.length === 0) {
      toast('No frames recorded');
      this.setState('welcome');
      return;
    }
    
    this.setState('editor');
    await this.initEditor();
    toast(`${this.recordedFrames.length} frames captured at ${this.captureFrameRate}fps`);
  }
  
  async initEditor() {
    // Small delay to ensure DOM is ready
    await new Promise(r => setTimeout(r, 100));
    
    const canvas = $('#preview-canvas');
    this.gifPreview = new GifPreview(canvas, this.recordedFrames);
    this.gifPreview.render();
    this.gifPreview.startAnimation(15);
    
    const timelineContainer = $('#timeline-container');
    console.log('Initializing timeline with container:', timelineContainer);
    console.log('Frame count:', this.recordedFrames.length);
    
    this.timeline = new Timeline(timelineContainer, this.recordedFrames);
    this.timeline.onTrim = (start, end) => {
      this.gifPreview.setRange(start, end);
      this.updateEstimate();
    };
    
    this.updateEstimate();
  }
  
  updateEstimate() {
    const format = $('#format-select').value;
    const fps = parseInt($('#fps-select').value);
    const quality = parseFloat($('#quality-select').value);
    const frameCount = this.timeline ? this.timeline.getFrameCount() : this.recordedFrames.length;
    
    let estimatedBytes;
    
    if (format === 'webp') {
      estimatedBytes = frameCount * 20000 * quality;
    } else if (format === 'mp4') {
      const duration = frameCount / this.captureFrameRate;
      const bitrate = 2500000 * quality;
      estimatedBytes = (bitrate / 8) * duration;
    } else {
      estimatedBytes = frameCount * 50000 * quality;
    }
    
    const estimatedMB = (estimatedBytes / 1024 / 1024).toFixed(1);
    $('#size-estimate').textContent = `~${estimatedMB} MB`;
  }
  
  async exportFile() {
    const format = $('#format-select').value;
    const fps = parseInt($('#fps-select').value);
    const quality = parseFloat($('#quality-select').value);
    const range = this.timeline.getRange();
    const exportFrames = this.recordedFrames.slice(range.start, range.end);
    
    if (exportFrames.length === 0) {
      toast('No frames to export');
      return;
    }
    
    this.showProgress('Preparing export...');
    
    try {
      if (format === 'webp') {
        await this.exportWebP(exportFrames, fps, quality);
      } else if (format === 'mp4') {
        await this.exportMP4(exportFrames, fps, quality);
      } else {
        await this.exportGIF(exportFrames, fps, quality);
      }
    } catch (err) {
      console.error('Export error:', err);
      toast('Export failed: ' + err.message);
      this.hideProgress();
    }
  }
  
  async exportWebP(frames, targetFPS, quality) {
    const canvas = document.createElement('canvas');
    canvas.width = frames[0].width;
    canvas.height = frames[0].height;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    
    const frameInterval = Math.round(this.captureFrameRate / targetFPS);
    const sampledFrames = frames.filter((_, i) => i % frameInterval === 0);
    
    const frameBuffers = [];
    
    for (let i = 0; i < sampledFrames.length; i++) {
      ctx.putImageData(sampledFrames[i], 0, 0);
      const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
      frameBuffers.push(imageData.data.buffer);
      
      this.updateProgress(`Processing frame ${i + 1}/${sampledFrames.length}`, (i + 1) / sampledFrames.length * 100);
      
      if (i % 3 === 0) {
        await new Promise(r => setTimeout(r, 0));
      }
    }
    
    this.updateProgress('Encoding WebP...', 100);
    await new Promise(r => setTimeout(r, 100));
    
    const result = await window.electronAPI.exportWebP({
      frames: frameBuffers,
      fps: targetFPS,
      width: canvas.width,
      height: canvas.height,
      quality: quality
    });
    
    this.hideProgress();
    
    if (result.success) {
      toast('Animated WebP exported successfully!');
    } else if (!result.cancelled) {
      toast('Export failed');
    }
  }
  
  async exportMP4(frames, targetFPS, quality) {
    const canvas = document.createElement('canvas');
    canvas.width = frames[0].width;
    canvas.height = frames[0].height;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    
    const frameInterval = Math.round(this.captureFrameRate / targetFPS);
    const sampledFrames = frames.filter((_, i) => i % frameInterval === 0);
    
    const frameBuffers = [];
    
    for (let i = 0; i < sampledFrames.length; i++) {
      ctx.putImageData(sampledFrames[i], 0, 0);
      const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
      frameBuffers.push(imageData.data.buffer);
      
      this.updateProgress(`Processing frame ${i + 1}/${sampledFrames.length}`, (i + 1) / sampledFrames.length * 100);
      
      if (i % 3 === 0) {
        await new Promise(r => setTimeout(r, 0));
      }
    }
    
    this.updateProgress('Encoding MP4...', 100);
    await new Promise(r => setTimeout(r, 100));
    
    const result = await window.electronAPI.exportMP4({
      frames: frameBuffers,
      fps: targetFPS,
      width: canvas.width,
      height: canvas.height,
      quality: quality
    });
    
    this.hideProgress();
    
    if (result.success) {
      toast('MP4 exported successfully!');
    } else if (!result.cancelled) {
      toast('Export failed');
    }
  }
  
  async exportGIF(frames, targetFPS, quality) {
    const canvas = document.createElement('canvas');
    canvas.width = frames[0].width;
    canvas.height = frames[0].height;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    
    const frameInterval = Math.round(this.captureFrameRate / targetFPS);
    const sampledFrames = frames.filter((_, i) => i % frameInterval === 0);
    
    const frameBuffers = [];
    
    for (let i = 0; i < sampledFrames.length; i++) {
      ctx.putImageData(sampledFrames[i], 0, 0);
      const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
      frameBuffers.push(imageData.data.buffer);
      
      this.updateProgress(`Processing frame ${i + 1}/${sampledFrames.length}`, (i + 1) / sampledFrames.length * 100);
      
      if (i % 3 === 0) {
        await new Promise(r => setTimeout(r, 0));
      }
    }
    
    this.updateProgress('Encoding GIF...', 100);
    await new Promise(r => setTimeout(r, 100));
    
    const result = await window.electronAPI.exportGIF({
      frames: frameBuffers,
      fps: targetFPS,
      width: canvas.width,
      height: canvas.height,
      quality: quality
    });
    
    this.hideProgress();
    
    if (result.success) {
      toast('GIF exported successfully!');
    } else if (!result.cancelled) {
      toast('Export failed');
    }
  }
  
  showProgress(text) {
    $('#progress-overlay').classList.remove('hidden');
    $('#progress-text').textContent = text;
    $('#progress-fill').style.width = '0%';
  }
  
  updateProgress(text, percent) {
    $('#progress-text').textContent = text;
    $('#progress-fill').style.width = `${percent}%`;
  }
  
  hideProgress() {
    $('#progress-overlay').classList.add('hidden');
  }
  
  setState(newState) {
    this.state = newState;
    $('#welcome-state').classList.toggle('hidden', newState !== 'welcome');
    $('#recording-state').classList.toggle('hidden', newState !== 'recording');
    $('#editor-state').classList.toggle('hidden', newState !== 'editor');
  }
}

new Recorder();