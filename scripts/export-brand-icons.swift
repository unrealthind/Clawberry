// macOS-only raster export of the repo-native RaaviClaw vector mark.
// Run: swift scripts/export-brand-icons.swift
import Foundation
import CoreGraphics
import ImageIO

let directory = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
    .appendingPathComponent("app/public")
for size in [16, 32, 180, 192, 512, 1024] {
    let context = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8,
        bytesPerRow: size * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    context.setFillColor(CGColor(red: 23/255, green: 25/255, blue: 28/255, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: size, height: size))
    context.translateBy(x: 0, y: CGFloat(size))
    context.scaleBy(x: CGFloat(size)/64, y: -CGFloat(size)/64)
    context.translateBy(x: 8, y: 8)
    context.scaleBy(x: 0.75, y: 0.75)
    let p = CGMutablePath()
    p.addRect(CGRect(x: 12, y: 10, width: 8, height: 44))
    p.move(to: CGPoint(x: 26, y: 10)); p.addLine(to: CGPoint(x: 36, y: 10))
    p.addCurve(to: CGPoint(x: 52, y: 25), control1: CGPoint(x: 46, y: 10), control2: CGPoint(x: 52, y: 16))
    p.addCurve(to: CGPoint(x: 41, y: 39), control1: CGPoint(x: 52, y: 32), control2: CGPoint(x: 48, y: 37))
    for point in [CGPoint(x: 54, y: 54), CGPoint(x: 43, y: 54), CGPoint(x: 26, y: 34), CGPoint(x: 26, y: 30), CGPoint(x: 36, y: 30)] { p.addLine(to: point) }
    p.addCurve(to: CGPoint(x: 44, y: 25), control1: CGPoint(x: 41, y: 30), control2: CGPoint(x: 44, y: 28))
    p.addCurve(to: CGPoint(x: 36, y: 18), control1: CGPoint(x: 44, y: 22), control2: CGPoint(x: 41, y: 18))
    p.addLine(to: CGPoint(x: 26, y: 18)); p.closeSubpath()
    context.setFillColor(CGColor(red: 245/255, green: 246/255, blue: 247/255, alpha: 1))
    context.addPath(p); context.fillPath()
    let filename = size == 180 ? "apple-touch-icon.png" : "brand/icon-\(size).png"
    let output = directory.appendingPathComponent(filename)
    let destination = CGImageDestinationCreateWithURL(output as CFURL, "public.png" as CFString, 1, nil)!
    CGImageDestinationAddImage(destination, context.makeImage()!, nil)
    guard CGImageDestinationFinalize(destination) else { fatalError("Icon export failed") }
    print(output.path)
}
