import Foundation

guard CommandLine.arguments.count > 1 else {
    print("{\"error\":\"no_session_key\"}")
    exit(1)
}

let token = CommandLine.arguments[1]
let sem = DispatchSemaphore(value: 0)
var result = "{\"error\":\"timeout\"}"

var req = URLRequest(url: URL(string: "https://claude.ai/api/organizations")!)
req.setValue("sessionKey=\(token)", forHTTPHeaderField: "Cookie")
req.setValue("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15", forHTTPHeaderField: "User-Agent")
req.timeoutInterval = 10

URLSession.shared.dataTask(with: req) { data, resp, err in
    guard let data = data,
          let json = try? JSONSerialization.jsonObject(with: data) as? [[String:Any]],
          let orgId = json.first?["uuid"] as? String else {
        result = "{\"error\":\"orgs_failed\"}"
        sem.signal()
        return
    }

    var req2 = URLRequest(url: URL(string: "https://claude.ai/api/organizations/\(orgId)/usage")!)
    req2.setValue("sessionKey=\(token)", forHTTPHeaderField: "Cookie")
    req2.setValue("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15", forHTTPHeaderField: "User-Agent")
    req2.timeoutInterval = 10

    URLSession.shared.dataTask(with: req2) { data2, _, _ in
        if let data2 = data2, let usage = String(data: data2, encoding: .utf8) {
            result = usage
        } else {
            result = "{\"error\":\"usage_failed\"}"
        }
        sem.signal()
    }.resume()
}.resume()

sem.wait()
print(result)
