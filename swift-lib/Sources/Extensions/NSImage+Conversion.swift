/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Cocoa
import CoreGraphics

extension NSImage {
    func png() -> Data? {
        tiffRepresentation?.bitmap?.png()
    }
    
    func resize(to size: Int) -> NSImage {
        let newSizeInt = size / Int(NSScreen.main?.backingScaleFactor ?? 1)
        let newSize = CGSize(width: newSizeInt, height: newSizeInt)
        
        let image = NSImage(size: newSize)
        image.lockFocus()
        NSGraphicsContext.current?.imageInterpolation = .high
        
        draw(
            in: CGRect(
                origin: .zero,
                size: newSize
            ),
            from: .zero,
            operation: .copy,
            fraction: 1
        )
        
        image.unlockFocus()
        return image
    }
}
