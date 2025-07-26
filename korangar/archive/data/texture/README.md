# How to create SDF icons

The in-game interface UI uses and also UI overlay uses signed distance function (SDF) based images to render sharp
icons. These
SDF icons allows us to scale the icons while them staying sharp.

To create new SDF, follow the following guide:

1. Create the icon using simple vector graphics (for example made with Inkscape) and export it as a plain SVG.
2. Simple shapes without small details will result in the best result later.
3. Use the following tool to create the final SDF based PNG files:
   https://chlumsky.github.io/msdfgen-web-ui/
4. Select as the distance field type: SDF
5. Select as output dimension for interface icons: 32x32. For the UI overlay icons 64x64.
6. Select as the base range: 4
7. Click on "Preview" and then "Save" to save the generated SDF

You can then store the generated PNG files in this texture folder and use them in the game code.
