const { app, BrowserWindow, ipcMain, dialog, screen } = require('electron');
const path = require('path');
const fs = require('fs');
const { spawn } = require('child_process');
const ffmpegPath = require('ffmpeg-static');

let mainWindow;
let overlayWindow = null;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  
  mainWindow.loadFile(path.join(__dirname, 'web/index.html'));
}

app.whenReady().then(createWindow);
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});
app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow();
});

ipcMain.handle('get-sources', async () => {
  const { desktopCapturer } = require('electron');
  const sources = await desktopCapturer.getSources({
    types: ['screen'], // Only screens, not windows
    thumbnailSize: { width: 300, height: 200 }
  });
  
  return sources.map(source => ({
    id: source.id,
    name: source.name,
    thumbnail: source.thumbnail.toDataURL()
  }));
});

ipcMain.handle('show-capture-overlay', async (event) => {
  const primaryDisplay = screen.getPrimaryDisplay();
  const { width, height } = primaryDisplay.bounds;
  
  overlayWindow = new BrowserWindow({
    width: width,
    height: height,
    x: 0,
    y: 0,
    frame: false,
    transparent: true,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  });
  
  overlayWindow.loadFile(path.join(__dirname, 'web/overlay.html'));
  overlayWindow.setIgnoreMouseEvents(false);
  
  return { success: true, screenWidth: width, screenHeight: height };
});

ipcMain.handle('capture-confirmed', async (event, bounds) => {
  if (overlayWindow) {
    overlayWindow.close();
    overlayWindow = null;
  }
  
  // Get the first screen source
  const { desktopCapturer } = require('electron');
  const sources = await desktopCapturer.getSources({ types: ['screen'] });
  const screenSource = sources[0];
  
  mainWindow.webContents.send('recording-source-ready', { 
    sourceId: screenSource.id, 
    bounds: bounds 
  });
  
  return { success: true };
});

ipcMain.handle('capture-cancelled', async (event) => {
  if (overlayWindow) {
    overlayWindow.close();
    overlayWindow = null;
  }
  mainWindow.webContents.send('capture-cancelled');
  return { success: true };
});

ipcMain.handle('start-recording', async (event, config) => {
  try {
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('stop-recording', async (event, frames) => {
  return { success: true, frames: frames || [] };
});

ipcMain.handle('export-webp', async (event, data) => {
  const { canceled, filePath } = await dialog.showSaveDialog(mainWindow, {
    defaultPath: 'recording.webp',
    filters: [{ name: 'Animated WebP', extensions: ['webp'] }]
  });
  
  if (canceled) return { success: false, cancelled: true };
  
  try {
    const { frames, fps, width, height, quality } = data;
    
    return new Promise((resolve, reject) => {
      const ffmpeg = spawn(ffmpegPath, [
        '-f', 'rawvideo',
        '-pix_fmt', 'rgba',
        '-s', `${width}x${height}`,
        '-r', String(fps),
        '-i', '-',
        '-c:v', 'libwebp',
        '-lossless', '0',
        '-compression_level', '6',
        '-quality', String(Math.round(quality * 100)),
        '-loop', '0',
        '-y',
        filePath
      ]);
      
      ffmpeg.on('error', (err) => {
        reject(err);
      });
      
      ffmpeg.stdin.on('error', (err) => {
        if (err.code !== 'EPIPE') {
          reject(err);
        }
      });
      
      ffmpeg.on('close', (code) => {
        if (code === 0) {
          resolve({ success: true, path: filePath });
        } else {
          reject(new Error(`FFmpeg exited with code ${code}`));
        }
      });
      
      // Write frames sequentially
      let frameIndex = 0;
      const writeNextFrame = () => {
        if (frameIndex < frames.length) {
          const success = ffmpeg.stdin.write(Buffer.from(frames[frameIndex]));
          frameIndex++;
          if (success) {
            setImmediate(writeNextFrame);
          } else {
            ffmpeg.stdin.once('drain', writeNextFrame);
          }
        } else {
          ffmpeg.stdin.end();
        }
      };
      
      writeNextFrame();
    });
    
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('export-mp4', async (event, data) => {
  const { canceled, filePath } = await dialog.showSaveDialog(mainWindow, {
    defaultPath: 'recording.mp4',
    filters: [{ name: 'MP4 Video', extensions: ['mp4'] }]
  });
  
  if (canceled) return { success: false, cancelled: true };
  
  try {
    const { frames, fps, width, height, quality } = data;
    const crf = Math.round(51 - (quality * 28));
    
    return new Promise((resolve, reject) => {
      const ffmpeg = spawn(ffmpegPath, [
        '-f', 'rawvideo',
        '-pix_fmt', 'rgba',
        '-s', `${width}x${height}`,
        '-r', String(fps),
        '-i', '-',
        '-c:v', 'libx264',
        '-preset', 'medium',
        '-crf', String(crf),
        '-pix_fmt', 'yuv420p',
        '-movflags', '+faststart',
        '-y',
        filePath
      ]);
      
      ffmpeg.on('error', (err) => {
        reject(err);
      });
      
      ffmpeg.stdin.on('error', (err) => {
        if (err.code !== 'EPIPE') {
          reject(err);
        }
      });
      
      ffmpeg.on('close', (code) => {
        if (code === 0) {
          resolve({ success: true, path: filePath });
        } else {
          reject(new Error(`FFmpeg exited with code ${code}`));
        }
      });
      
      // Write frames sequentially to avoid EPIPE
      let frameIndex = 0;
      const writeNextFrame = () => {
        if (frameIndex < frames.length) {
          const success = ffmpeg.stdin.write(Buffer.from(frames[frameIndex]));
          frameIndex++;
          if (success) {
            setImmediate(writeNextFrame);
          } else {
            ffmpeg.stdin.once('drain', writeNextFrame);
          }
        } else {
          ffmpeg.stdin.end();
        }
      };
      
      writeNextFrame();
    });
    
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('export-gif', async (event, data) => {
  const { canceled, filePath } = await dialog.showSaveDialog(mainWindow, {
    defaultPath: 'recording.gif',
    filters: [{ name: 'GIF', extensions: ['gif'] }]
  });
  
  if (canceled) return { success: false, cancelled: true };
  
  try {
    const { frames, fps, width, height, quality } = data;
    
    return new Promise((resolve, reject) => {
      const ffmpeg = spawn(ffmpegPath, [
        '-f', 'rawvideo',
        '-pix_fmt', 'rgba',
        '-s', `${width}x${height}`,
        '-r', String(fps),
        '-i', '-',
        '-vf', `fps=${fps},scale=${width}:${height}:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=256[p];[s1][p]paletteuse=dither=bayer`,
        '-loop', '0',
        '-y',
        filePath
      ]);
      
      ffmpeg.on('error', (err) => {
        reject(err);
      });
      
      ffmpeg.stdin.on('error', (err) => {
        if (err.code !== 'EPIPE') {
          reject(err);
        }
      });
      
      ffmpeg.on('close', (code) => {
        if (code === 0) {
          resolve({ success: true, path: filePath });
        } else {
          reject(new Error(`FFmpeg exited with code ${code}`));
        }
      });
      
      // Write frames sequentially
      let frameIndex = 0;
      const writeNextFrame = () => {
        if (frameIndex < frames.length) {
          const success = ffmpeg.stdin.write(Buffer.from(frames[frameIndex]));
          frameIndex++;
          if (success) {
            setImmediate(writeNextFrame);
          } else {
            ffmpeg.stdin.once('drain', writeNextFrame);
          }
        } else {
          ffmpeg.stdin.end();
        }
      };
      
      writeNextFrame();
    });
    
  } catch (error) {
    return { success: false, error: error.message };
  }
});