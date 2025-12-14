//
//  QRScannerView.swift
//  PaykitDemo
//
//  QR code scanner view for scanning Paykit URIs
//

import SwiftUI
import AVFoundation

struct QRScannerView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var appState: AppState
    @StateObject private var scanner = QRScannerViewModel()
    @State private var showingResult = false
    @State private var scannedResult: ScannedUri?
    @State private var showingPaymentView = false
    @State private var paymentRecipientPubkey: String?
    
    var body: some View {
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
                // Handle the result based on type
                handleResult(result)
            }
            Button("Cancel", role: .cancel) {
                dismiss()
            }
        } message: { result in
            Text(resultDescription(result))
        }
        .sheet(isPresented: $showingPaymentView) {
            if let pubkey = paymentRecipientPubkey {
                PaymentView(initialRecipient: pubkey)
            }
        }
    }
    
    private func handleScannedCode(_ code: String) {
        // Check if it's a Paykit URI
        guard appState.paykitClient.isPaykitQR(data: code) else {
            return
        }
        
        // Parse it
        if let result = appState.paykitClient.parseScannedQR(data: code) {
            scannedResult = result
            showingResult = true
            scanner.stopScanning()
        }
    }
    
    private func handleResult(_ result: ScannedUri) {
        // Navigate to appropriate flow based on URI type
        switch result.uriType {
        case .pubky:
            if let pubkey = result.publicKey {
                // Navigate to payment flow with this public key
                paymentRecipientPubkey = pubkey
                showingPaymentView = true
            }
        case .invoice:
            if let methodId = result.methodId, let data = result.data {
                // Process invoice - navigate to payment view with invoice data
                // For now, show alert with invoice info
                print("Scanned Invoice: method=\(methodId), data=\(data)")
                dismiss()
            }
        case .paymentRequest:
            if let requestId = result.requestId {
                // Handle payment request - could navigate to payment request view
                print("Scanned Payment Request: \(requestId)")
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

// MARK: - QR Scanner View Model

class QRScannerViewModel: NSObject, ObservableObject, AVCaptureMetadataOutputObjectsDelegate {
    @Published var scannedCode: String?
    
    private var captureSession: AVCaptureSession?
    private var previewLayer: AVCaptureVideoPreviewLayer?
    
    func startScanning() {
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
        
        if captureSession.canAddInput(videoInput) {
            captureSession.addInput(videoInput)
        } else {
            print("Cannot add video input")
            return
        }
        
        let metadataOutput = AVCaptureMetadataOutput()
        
        if captureSession.canAddOutput(metadataOutput) {
            captureSession.addOutput(metadataOutput)
            
            metadataOutput.setMetadataObjectsDelegate(self, queue: DispatchQueue.main)
            metadataOutput.metadataObjectTypes = [.qr]
        } else {
            print("Cannot add metadata output")
            return
        }
        
        captureSession.startRunning()
    }
    
    func stopScanning() {
        captureSession?.stopRunning()
    }
    
    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject], from connection: AVCaptureConnection) {
        if let metadataObject = metadataObjects.first {
            guard let readableObject = metadataObject as? AVMetadataMachineReadableCodeObject else { return }
            guard let stringValue = readableObject.stringValue else { return }
            
            // Only process if we haven't already scanned this code
            if scannedCode != stringValue {
                scannedCode = stringValue
            }
        }
    }
}

// MARK: - Camera Preview

struct QRScannerPreview: UIViewControllerRepresentable {
    let scanner: QRScannerViewModel
    
    func makeUIViewController(context: Context) -> QRScannerViewController {
        let controller = QRScannerViewController()
        controller.scanner = scanner
        return controller
    }
    
    func updateUIViewController(_ uiViewController: QRScannerViewController, context: Context) {}
}

class QRScannerViewController: UIViewController {
    var scanner: QRScannerViewModel?
    private var previewLayer: AVCaptureVideoPreviewLayer?
    
    override func viewDidLoad() {
        super.viewDidLoad()
        setupPreview()
    }
    
    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        previewLayer?.frame = view.bounds
    }
    
    private func setupPreview() {
        guard let captureSession = scanner?.captureSession else { return }
        
        let previewLayer = AVCaptureVideoPreviewLayer(session: captureSession)
        previewLayer.videoGravity = .resizeAspectFill
        self.previewLayer = previewLayer
        
        view.layer.addSublayer(previewLayer)
    }
}

