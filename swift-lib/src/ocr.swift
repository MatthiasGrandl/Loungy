import Carbon.HIToolbox
import Cocoa
import CoreGraphics
import SwiftRs
import Vision

@_cdecl("ocr")
public func ocr(path: SRString) {
    let url = URL(fileURLWithPath: path.toString())
    guard let ciImage = CIImage(contentsOf: url) else {
        return
    }

    // Create a new image-request handler.
    let requestHandler = VNImageRequestHandler(ciImage: ciImage)

    // Create a new request to recognize text.
    let request = VNRecognizeTextRequest(completionHandler: recognizeTextHandler)

    do {
        // Perform the text-recognition request.
        try requestHandler.perform([request])
    } catch {
        print("Unable to perform the requests: \(error).")
    }
}

func recognizeTextHandler(request: VNRequest, error _: Error?) {
    guard let observations =
        request.results as? [VNRecognizedTextObservation]
    else {
        return
    }
    let recognizedStrings = observations.compactMap { observation in
        observation.topCandidates(1).first?.string
    }

    let pasteboard = NSPasteboard.general
    pasteboard.declareTypes([.string], owner: nil)
    pasteboard.setString(recognizedStrings.joined(separator: "\n"), forType: .string)
}
