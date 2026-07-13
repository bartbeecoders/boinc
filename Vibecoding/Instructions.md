Boinc is a file conversion utility that converts files from one format to another.

User will select a file in the standard os file browser, boinc submenu will appear in the context menu.
User will select the conversion option and the file will be converted.

App will be installed as a OS extension/service and will be available in the traybar.


## Main features:
- Convert files from one format to another
- Will support multiple conversion formats,
- Web portal, will provide a landing page (hosted on boinc.hideterms.com), where users can download the app

### Conversions to support:
- PDF to DOCX
- DOCX to PDF
- PNG to JPG
- JPG to PNG

Conversion system needs to be easily extensible to add new conversion formats.

## Architecture

- rust with minimal UI - use FLoem for the UI.
(https://lap.dev/floem/)
- should work on linux, windows and mac
- should be able to be installed as a OS extension

Create a detailed plan (save in plan.md) with phases and tasks.


Convert the Boinc website to react/vite


I'm on EndeavourOS, can you add to the dev.sh script the shortcuts on the file system. (cinnamon)