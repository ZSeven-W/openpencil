import Foundation

extension OpEngineHost {
    /// Defer the upcall and deliver data only on the live engine's owner thread.
    func deferRemoteImageRequest(requestID: UInt64, url: String) {
        DispatchQueue.main.async { [weak self] in
            guard let self, let engine = self.engine, self.view?.didTearDown != true,
                  let remoteURL = URL(string: url) else { return }
            let expectedEngine = UInt(bitPattern: engine)
            let task = self.remoteImageSession.dataTask(with: remoteURL) { [weak self] data, _, error in
                guard let data, error == nil else { return }
                DispatchQueue.main.async {
                    guard let self, let engine = self.engine, self.view?.didTearDown != true,
                          UInt(bitPattern: engine) == expectedEngine else { return }
                    let status = data.withUnsafeBytes { bytes in
                        op_remote_image_result(
                            engine, requestID, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count
                        )
                    }
                    if status == OpStatus_Ok { self.requestImmediateFrame() }
                }
            }
            task.resume()
        }
    }
}
