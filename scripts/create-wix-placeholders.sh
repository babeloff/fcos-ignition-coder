#!/bin/bash

# Script to create minimal placeholder files for WiX MSI installer
# These are basic placeholders - replace with actual branded assets for production

set -e

echo "🎨 Creating placeholder WiX installer assets..."

# Create wix directory if it doesn't exist
mkdir -p wix

# Create a minimal ICO file using ImageMagick (if available) or a placeholder
if command -v magick &> /dev/null; then
    echo "  📐 Using ImageMagick to create icon..."
    magick -size 32x32 canvas:blue -fill white -draw "rectangle 0,0 31,1" -draw "rectangle 0,0 1,31" -draw "rectangle 30,0 31,31" -draw "rectangle 0,30 31,31" wix/fcos-ignition-coder.ico
elif command -v convert &> /dev/null; then
    echo "  📐 Using ImageMagick (legacy) to create icon..."
    convert -size 32x32 xc:blue -fill white -draw "rectangle 0,0 31,1" -draw "rectangle 0,0 1,31" -draw "rectangle 30,0 31,31" -draw "rectangle 0,30 31,31" wix/fcos-ignition-coder.ico
else
    echo "  📄 Creating placeholder icon (install ImageMagick for better quality)..."
    # Create a minimal 1x1 pixel ICO placeholder that Windows will accept
    echo -e "\x00\x00\x01\x00\x01\x00\x01\x01\x00\x00\x01\x00\x18\x00\x30\x00\x00\x00\x16\x00\x00\x00\x28\x00\x00\x00\x01\x00\x00\x00\x02\x00\x00\x00\x01\x00\x18\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xFF\x00\x00\x00\x00\x00" > wix/fcos-ignition-coder.ico
fi

# Create banner BMP (493x58)
if command -v magick &> /dev/null; then
    echo "  📐 Creating banner image..."
    magick -size 493x58 gradient:blue-white wix/Banner.bmp
elif command -v convert &> /dev/null; then
    echo "  📐 Creating banner image (legacy)..."
    convert -size 493x58 gradient:blue-white wix/Banner.bmp
else
    echo "  📄 Creating placeholder banner..."
    # Create minimal BMP header for 493x58 image (will appear as solid color)
    python3 -c "
import struct
width, height = 493, 58
row_size = ((width * 3 + 3) // 4) * 4
image_size = row_size * height
file_size = 54 + image_size

with open('wix/Banner.bmp', 'wb') as f:
    # BMP file header
    f.write(b'BM')
    f.write(struct.pack('<I', file_size))
    f.write(b'\x00\x00\x00\x00')
    f.write(b'\x36\x00\x00\x00')

    # DIB header
    f.write(b'\x28\x00\x00\x00')
    f.write(struct.pack('<I', width))
    f.write(struct.pack('<I', height))
    f.write(b'\x01\x00')
    f.write(b'\x18\x00')
    f.write(b'\x00\x00\x00\x00')
    f.write(struct.pack('<I', image_size))
    f.write(b'\x12\x0B\x00\x00')
    f.write(b'\x12\x0B\x00\x00')
    f.write(b'\x00\x00\x00\x00')
    f.write(b'\x00\x00\x00\x00')

    # Pixel data (light blue gradient)
    for y in range(height):
        row = bytearray()
        for x in range(width):
            intensity = min(255, int(200 + (x * 55 / width)))
            row.extend([intensity, 200, 255])  # BGR format
        # Pad to multiple of 4 bytes
        while len(row) % 4 != 0:
            row.append(0)
        f.write(row)
" 2>/dev/null || {
        echo "  ⚠️  Python not available, creating minimal banner..."
        # Just create a minimal valid BMP file
        echo -e "\x42\x4D\x36\x00\x00\x00\x00\x00\x00\x00\x36\x00\x00\x00\x28\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01\x00\x18\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xFF\xFF\xFF" > wix/Banner.bmp
    }
fi

# Create dialog BMP (493x312)
if command -v magick &> /dev/null; then
    echo "  📐 Creating dialog background..."
    magick -size 493x312 canvas:"#F0F0FF" wix/Dialog.bmp
elif command -v convert &> /dev/null; then
    echo "  📐 Creating dialog background (legacy)..."
    convert -size 493x312 -background "#F0F0FF" -fill "#E0E0FF" -draw "rectangle 0,0 493,50" wix/Dialog.bmp
else
    echo "  📄 Creating placeholder dialog background..."
    python3 -c "
import struct
width, height = 493, 312
row_size = ((width * 3 + 3) // 4) * 4
image_size = row_size * height
file_size = 54 + image_size

with open('wix/Dialog.bmp', 'wb') as f:
    # BMP file header
    f.write(b'BM')
    f.write(struct.pack('<I', file_size))
    f.write(b'\x00\x00\x00\x00')
    f.write(b'\x36\x00\x00\x00')

    # DIB header
    f.write(b'\x28\x00\x00\x00')
    f.write(struct.pack('<I', width))
    f.write(struct.pack('<I', height))
    f.write(b'\x01\x00')
    f.write(b'\x18\x00')
    f.write(b'\x00\x00\x00\x00')
    f.write(struct.pack('<I', image_size))
    f.write(b'\x12\x0B\x00\x00')
    f.write(b'\x12\x0B\x00\x00')
    f.write(b'\x00\x00\x00\x00')
    f.write(b'\x00\x00\x00\x00')

    # Pixel data (light background)
    for y in range(height):
        row = bytearray()
        for x in range(width):
            # Light blue background
            row.extend([255, 240, 240])  # BGR format
        # Pad to multiple of 4 bytes
        while len(row) % 4 != 0:
            row.append(0)
        f.write(row)
" 2>/dev/null || {
        echo "  ⚠️  Python not available, creating minimal dialog background..."
        echo -e "\x42\x4D\x36\x00\x00\x00\x00\x00\x00\x00\x36\x00\x00\x00\x28\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01\x00\x18\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xFF\xFF\xFF" > wix/Dialog.bmp
    }
fi

echo "✅ WiX placeholder assets created successfully!"
echo ""
echo "📁 Created files:"
echo "   - wix/fcos-ignition-coder.ico (application icon)"
echo "   - wix/Banner.bmp (493x58 installer banner)"
echo "   - wix/Dialog.bmp (493x312 installer dialog background)"
echo "   - wix/License.rtf (license text for installer)"
echo ""
echo "📝 Note: These are placeholder assets for development."
echo "   For production releases, replace with professionally designed:"
echo "   - Branded application icon (32x32 ICO format)"
echo "   - Custom banner image (493x58 BMP, 24-bit color)"
echo "   - Custom dialog background (493x312 BMP, 24-bit color)"
echo ""
echo "💡 Tip: Use tools like GIMP, Photoshop, or online converters to create"
echo "   professional-quality installer assets matching your brand."
