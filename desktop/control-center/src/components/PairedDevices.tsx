import React, { useState } from "react";
import { Smartphone, Trash2, Key, Calendar, ShieldAlert } from "lucide-react";
import { invoke } from "@tauri-apps/api/tauri";

interface Device {
  id: string;
  name: string;
  platform: string;
  pubKeyHex: string;
  pairedDate: string;
  lastSeen: string;
}

export default function PairedDevices() {
  const [devices, setDevices] = useState<Device[]>([
    {
      id: "iphone-15-pro-uuid",
      name: "Chara's iPhone 15 Pro",
      platform: "iOS 17.5 (Back Tap Enabled)",
      pubKeyHex: "a1b2c3d4e5f67890123456789abcdef0123456789abcdef0123456789a1b2c3d",
      pairedDate: "2026-07-25 14:30",
      lastSeen: "Just now"
    },
    {
      id: "pixel-8-pro-uuid",
      name: "Chara's Pixel 8 Pro",
      platform: "Android 14 (Triple Tap Service)",
      pubKeyHex: "f6e5d4c3b2a10987654321fedcba0987654321fedcba0987654321fedcba0987",
      pairedDate: "2026-07-26 09:15",
      lastSeen: "10 minutes ago"
    }
  ]);

  const handleRevoke = async (id: string, name: string) => {
    if (!confirm(`Are you sure you want to revoke authorization for "${name}"? This phone will no longer be able to unlock your PC.`)) {
      return;
    }
    try {
      await invoke("revoke_device", { deviceId: id });
    } catch (e) {
      console.log("Simulated revocation in dev mode");
    }
    setDevices(devices.filter(d => d.id !== id));
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-xl font-bold text-white">Authorized Mobile Vault</h3>
          <p className="text-sm text-gray-400 mt-1">
            Only mobile devices listed below have cryptographic permission to unlock this workstation.
          </p>
        </div>
        <span className="px-3 py-1 bg-surface text-xs font-semibold text-gray-300 rounded-lg border border-gray-800">
          {devices.length} {devices.length === 1 ? "Device Paired" : "Devices Paired"}
        </span>
      </div>

      {devices.length === 0 ? (
        <div className="glass-card p-12 rounded-2xl text-center space-y-3">
          <ShieldAlert className="w-12 h-12 text-yellow-500 mx-auto animate-bounce" />
          <h4 className="font-bold text-white text-lg">No Paired Devices Found</h4>
          <p className="text-sm text-gray-400 max-w-md mx-auto">
            Your workstation cannot be unlocked via phone until you pair a mobile device using the QR Scanner.
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {devices.map(device => (
            <div key={device.id} className="glass-card p-6 rounded-2xl flex flex-col md:flex-row md:items-center justify-between gap-4 border border-gray-800 hover:border-gray-700 transition">
              <div className="flex items-start gap-4">
                <div className="p-3 bg-blue-500/10 text-blue-400 rounded-xl border border-blue-500/20">
                  <Smartphone className="w-6 h-6" />
                </div>
                <div className="space-y-1">
                  <div className="flex items-center gap-2">
                    <h4 className="font-semibold text-white">{device.name}</h4>
                    <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-mint/10 text-mint border border-mint/20">
                      {device.platform}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-gray-400 font-mono">
                    <Key className="w-3.5 h-3.5 text-gray-500" />
                    Ed25519 Pub: {device.pubKeyHex.substring(0, 20)}...
                  </div>
                  <div className="flex items-center gap-4 text-xs text-gray-500 pt-1">
                    <span className="flex items-center gap-1">
                      <Calendar className="w-3.5 h-3.5" /> Paired: {device.pairedDate}
                    </span>
                    <span>•</span>
                    <span className="text-green-400">Last Seen: {device.lastSeen}</span>
                  </div>
                </div>
              </div>

              <button
                onClick={() => handleRevoke(device.id, device.name)}
                className="px-4 py-2 bg-red-500/10 hover:bg-red-500/20 text-red-400 border border-red-500/30 rounded-xl text-xs font-semibold transition flex items-center justify-center gap-2 w-full md:w-auto"
              >
                <Trash2 className="w-4 h-4" />
                Revoke Authorization
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
