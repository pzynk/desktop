import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const targetDir = path.resolve(__dirname, '../src-tauri/resources');
const targetFile = path.join(targetDir, 'obs-virtualcam-module64.dll');

if (fs.existsSync(targetFile)) {
  console.log(`[setup-resources] obs-virtualcam-module64.dll already exists at ${targetFile}`);
  process.exit(0);
}

console.log('[setup-resources] obs-virtualcam-module64.dll is missing.');
console.log('[setup-resources] Downloading OBS Studio ZIP to extract the virtual camera DLL...');

const tempZip = path.join(__dirname, 'obs-temp.zip');
const url = 'https://github.com/obsproject/obs-studio/releases/download/30.1.2/OBS-Studio-30.1.2.zip';
const dllZipPath = 'data/obs-plugins/win-dshow/obs-virtualcam-module64.dll';

try {
  // Ensure target directory exists
  if (!fs.existsSync(targetDir)) {
    fs.mkdirSync(targetDir, { recursive: true });
  }

  // Download the ZIP file using curl
  console.log(`[setup-resources] Downloading from: ${url}`);
  execSync(`curl -L "${url}" -o "${tempZip}"`, { stdio: 'inherit' });

  // Extract the specific DLL using tar
  console.log(`[setup-resources] Extracting ${dllZipPath}...`);
  execSync(`tar -xf "${tempZip}" "${dllZipPath}"`, { cwd: __dirname });

  // Move the DLL to the destination
  const extractedDll = path.join(__dirname, 'data/obs-plugins/win-dshow/obs-virtualcam-module64.dll');
  if (fs.existsSync(extractedDll)) {
    fs.renameSync(extractedDll, targetFile);
    console.log(`[setup-resources] Successfully installed DLL to ${targetFile}`);
  } else {
    throw new Error('Extracted DLL file not found. Extraction might have failed.');
  }
} catch (error) {
  console.error('[setup-resources] Error occurred during setup:', error.message);
  process.exit(1);
} finally {
  // Clean up files
  if (fs.existsSync(tempZip)) {
    try {
      fs.unlinkSync(tempZip);
    } catch (e) {
      console.warn('[setup-resources] Failed to delete temporary ZIP:', e.message);
    }
  }
  const extractedDataDir = path.join(__dirname, 'data');
  if (fs.existsSync(extractedDataDir)) {
    try {
      fs.rmSync(extractedDataDir, { recursive: true, force: true });
    } catch (e) {
      console.warn('[setup-resources] Failed to delete extracted data directory:', e.message);
    }
  }
}
