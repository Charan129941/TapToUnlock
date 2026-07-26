import React, { useState } from "react";
import { CheckCircle2, XCircle, ShieldAlert, Wifi, Bluetooth, Radio } from "lucide-react";

interface LogEvent {
  id: string;
  timestamp: string;
  device: string;
  action: string;
  channel: "Wi-Fi mTLS" | "BLE GATT" | "mDNS";
  status: "SUCCESS" | "REJECTED_REPLAY" | "REJECTED_SIG";
  details: string;
}

export default function AuditLogs() {
  const [logs] = useState<LogEvent[]>([
    {
      id: "log-1",
      timestamp: "Today, 10:14:22 AM",
      device: "Chara's iPhone 15 Pro",
      action: "UNLOCK_SESSION",
      channel: "Wi-Fi mTLS",
      status: "SUCCESS",
      details: "Ed25519 signature verified. Nonce #4092 accepted."
    },
    {
      id: "log-2",
      timestamp: "Today, 09:30:05 AM",
      device: "Chara's Pixel 8 Pro",
      action: "UNLOCK_SESSION",
      channel: "BLE GATT",
      status: "SUCCESS",
      details: "Triple Tap gesture authenticated via Android BiometricPrompt."
    },
    {
      id: "log-3",
      timestamp: "Yesterday, 11:45:10 PM",
      device: "Unknown Device (IP: 192.168.1.188)",
      action: "UNLOCK_SESSION",
      channel: "Wi-Fi mTLS",
      status: "REJECTED_SIG",
      details: "Cryptographic verification failed. Public key not in authorized vault!"
    },
    {
      id: "log-4",
      timestamp: "Yesterday, 06:20:19 PM",
      device: "Chara's iPhone 15 Pro",
      action: "LOCK_SESSION",
      channel: "Wi-Fi mTLS",
      status: "SUCCESS",
      details: "Double Tap gesture executed lock challenge."
    }
  ]);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-xl font-bold text-white">Security Audit Logs</h3>
          <p className="text-sm text-gray-400 mt-1">
            Real-time chronological record of every biometric unlock attempt and wireless handshake.
          </p>
        </div>
      </div>

      <div className="glass-card rounded-2xl border border-gray-800 overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-gray-800 bg-surface/50 text-xs font-semibold text-gray-400 uppercase tracking-wider">
                <th className="py-4 px-6">Status</th>
                <th className="py-4 px-6">Timestamp</th>
                <th className="py-4 px-6">Device</th>
                <th className="py-4 px-6">Action</th>
                <th className="py-4 px-6">Channel</th>
                <th className="py-4 px-6">Audit Details</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-800/60 text-sm">
              {logs.map(log => (
                <tr key={log.id} className="hover:bg-surface-light/30 transition">
                  <td className="py-4 px-6 whitespace-nowrap">
                    {log.status === "SUCCESS" ? (
                      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-green-500/10 text-green-400 border border-green-500/20">
                        <CheckCircle2 className="w-3.5 h-3.5" />
                        Verified
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-red-500/10 text-red-400 border border-red-500/20">
                        <XCircle className="w-3.5 h-3.5" />
                        Rejected
                      </span>
                    )}
                  </td>
                  <td className="py-4 px-6 text-gray-400 font-mono text-xs whitespace-nowrap">{log.timestamp}</td>
                  <td className="py-4 px-6 font-medium text-white whitespace-nowrap">{log.device}</td>
                  <td className="py-4 px-6 whitespace-nowrap">
                    <span className="font-mono text-xs text-mint bg-background px-2 py-1 rounded border border-gray-800">
                      {log.action}
                    </span>
                  </td>
                  <td className="py-4 px-6 whitespace-nowrap">
                    <span className="inline-flex items-center gap-1.5 text-xs text-gray-300">
                      {log.channel === "Wi-Fi mTLS" ? <Wifi className="w-3.5 h-3.5 text-blue-400" /> : <Bluetooth className="w-3.5 h-3.5 text-purple-400" />}
                      {log.channel}
                    </span>
                  </td>
                  <td className="py-4 px-6 text-xs text-gray-400 max-w-xs truncate">{log.details}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
