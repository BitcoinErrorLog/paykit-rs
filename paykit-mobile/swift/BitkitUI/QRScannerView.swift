//
//  QRScannerView.swift
//  PaykitMobile
//
//  QR Scanner UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import AVFoundation
import PaykitMobile

/// QR Scanner view model for Bitkit integration
public class BitkitQRScannerViewModel: NSObject, ObservableObject, AVCaptureMetadataOutputObjectsDelegate {
    @Published public var scannedCode: String?
    
    var captureSession: AVCaptureSession?
    private var previewLayer: AVCaptureVideoPreviewLayer?
    
    public func startScanning() {
        guard let videoCaptureDevice = AVCaptureDevice.default(for: .video) else {
            print("Failed to get video capture device")
            return
        }
        
        let videoInput: AVCaptureDeviceInput
        
        do {
            videoInput = try AVCaptureDeviceInput(device: videoCaptureDevice)
        } catch {
            print("Failed to create video input: \(error)")
            return
        }
        
        let captureSession = AVCaptureSession()
        self.captureSession = captureSession
        
        captureSession.addInput(videoInput)
        
        let metadataOutput = AVCaptureMetadataOutput()
        captureSession.addOutput(metadataOutput)
        
        metadataOutput.setMetadataObjectsDelegate(self, queue: DispatchQueue.main)
        metadataOutput.metadataObjectTypes = [.qr]
        
        captureSession.startRunning()
    }
    
    public func stopScanning() {
        captureSession?.stopRunning()
    }
    
    public func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject], from connection: AVCaptureConnection) {
        if let metadataObject = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
           let stringValue = metadataObject.stringValue {
            scannedCode = stringValue
        }
    }
}

/// QR Scanner view component
public struct BitkitQRScannerView: View {
    @Environment(\.dismiss) private var dismiss
    @StateObject private var scanner = BitkitQRScannerViewModel()
    @State private var showingResult = false
    @State private var scannedResult: ScannedUri?
    
    private let paykitClient: PaykitClient
    
    // Navigation callbacks
    public var onScannedPubky: ((String) -> Void)?
    public var onScannedInvoice: ((String, String) -> Void)?
    public var onScannedPaymentRequest: ((String) -> Void)?
    
    public init(
        paykitClient: PaykitClient,
        onScannedPubky: ((String) -> Void)? = nil,
        onScannedInvoice: ((String, String) -> Void)? = nil,
        onScannedPaymentRequest: ((String) -> Void)? = nil
    ) {
        self.paykitClient = paykitClient
        self.onScannedPubky = onScannedPubky
        self.onScannedInvoice = onScannedInvoice
        self.onScannedPaymentRequest = onScannedPaymentRequest
    }
    
    public var body: some View {
        ZStack {
            // Camera preview
            QRScannerPreview(scanner: scanner)
                .ignoresSafeArea()
            
            // Overlay
            VStack {
                Spacer()
                
                VStack(spacing: 16) {
                    Text("Position QR code within frame")
                        .foregroundColor(.white)
                        .padding()
                        .background(Color.black.opacity(0.7))
                        .cornerRadius(8)
                    
                    Button("Cancel") {
                        dismiss()
                    }
                    .buttonStyle(.borderedProminent)
                }
                .padding()
            }
        }
        .onAppear {
            scanner.startScanning()
        }
        .onDisappear {
            scanner.stopScanning()
        }
        .onChange(of: scanner.scannedCode) { newValue in
            if let code = newValue {
                handleScannedCode(code)
            }
        }
        .alert("Scanned QR Code", isPresented: $showingResult, presenting: scannedResult) { result in
            Button("OK") {
                handleResult(result)
            }
            Button("Cancel", role: .cancel) {
                dismiss()
            }
        } message: { result in
            Text(resultDescription(result))
        }
    }
    
    private func handleScannedCode(_ code: String) {
        // Check if it's a Paykit URI
        guard paykitClient.isPaykitQR(data: code) else {
            return
        }
        
        // Parse it
        if let result = paykitClient.parseScannedQR(data: code) {
            scannedResult = result
            showingResult = true
            scanner.stopScanning()
        }
    }
    
    private func handleResult(_ result: ScannedUri) {
        switch result.uriType {
        case .pubky:
            if let pubkey = result.publicKey {
                onScannedPubky?(pubkey)
                dismiss()
            }
        case .invoice:
            if let methodId = result.methodId, let data = result.data {
                onScannedInvoice?(methodId, data)
                dismiss()
            }
        case .paymentRequest:
            if let requestId = result.requestId {
                onScannedPaymentRequest?(requestId)
                dismiss()
            }
        case .unknown:
            dismiss()
        }
    }
    
    private func resultDescription(_ result: ScannedUri) -> String {
        switch result.uriType {
        case .pubky:
            return "Pubky URI detected. Public key: \(result.publicKey ?? "unknown")"
        case .invoice:
            return "Invoice detected. Method: \(result.methodId ?? "unknown")"
        case .paymentRequest:
            return "Payment Request detected. ID: \(result.requestId ?? "unknown")"
        case .unknown:
            return "Unknown QR code format"
        }
    }
}

// MARK: - QR Scanner Preview

struct QRScannerPreview: UIViewControllerRepresentable {
    let scanner: BitkitQRScannerViewModel
    
    func makeUIViewController(context: Context) -> QRScannerViewController {
        let controller = QRScannerViewController()
        controller.scanner = scanner
        return controller
    }
    
    func updateUIViewController(_ uiViewController: QRScannerViewController, context: Context) {
        // Update if needed
    }
}

class QRScannerViewController: UIViewController {
    var scanner: BitkitQRScannerViewModel?
    var previewLayer: AVCaptureVideoPreviewLayer?
    
    override func viewDidLoad() {
        super.viewDidLoad()
        
        guard let captureSession = scanner?.captureSession else { return }
        
        let previewLayer = AVCaptureVideoPreviewLayer(session: captureSession)
        previewLayer.frame = view.layer.bounds
        previewLayer.videoGravity = .resizeAspectFill
        view.layer.addSublayer(previewLayer)
        self.previewLayer = previewLayer
    }
    
    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        previewLayer?.frame = view.layer.bounds
    }
}
