/*
 This source file is part of the Loungy open source project
 
 Copyright (c) 2024 Loungy, Matthias Grandl and the Loungy project contributors
 Licensed under MIT License
 
 See https://github.com/MatthiasGrandl/Loungy/blob/main/LICENSE.md for license information
 */

import Carbon.HIToolbox
import Cocoa
import CoreGraphics
import Vision

final class OCR {

    func readText(from path: String) {
        let url = URL(fileURLWithPath: path)

        guard let ciImage = CIImage(contentsOf: url) else {
            return
        }
        
        // Create a new image-request handler.
        let requestHandler = VNImageRequestHandler(ciImage: ciImage)
        
        // Create a new request to recognize text.
        let request = VNRecognizeTextRequest(
            completionHandler: recognizeTextHandler
        )
        
        do {
            // Perform the text-recognition request.
            try requestHandler.perform([request])
        } 
        catch {
            print("Unable to perform the requests: \(error).")
        }
    }
    
    private func recognizeTextHandler(request: VNRequest, error _: Error?) {
        guard let observations = request.results as? [VNRecognizedTextObservation] else {
            return
        }
        
        let recognizedStrings = observations.compactMap { observation -> String? in
            guard let candidate = observation.topCandidates(1).first else {
                return nil
            }
            
            return candidate.string
        }
        
        let pasteboard = NSPasteboard.general
        pasteboard.declareTypes([.string], owner: nil)
        pasteboard.setString(
            recognizedStrings.joined(separator: "\n"),
            forType: .string
        )
    }
}
