import React, { useState, useEffect } from "react";
import { QrCode, RefreshCw, Shield, CheckCircle } from "lucide-react";
import { QRCodeSVG } from "qrcode.react";

export default function PairingQrModal() {
  const [pairingUri, setPairingUri] = useState("");
  const [pin, setPin] = useState("849201");
  const [timeLeft, setTimeLeft] = useState(300);

  const generateNewPairingSession = () => {
    const randomPin = Math.floor(100000 + Math.random() * 900000).toString();
    const mockUri = `opentap://pair?pc_uuid=chara-workstation&pub_key=a1b2c3d4e5f6&ip=192.168.1.100&port=8765&pin=${randomPin}`;
    setPin(randomPin);
    setPairingUri(mockUri);
    setTimeLeft(300);
  };

  useEffect(() => {
    generateNewPairingSession();
    const interval = setInterval(() => {
      setTimeLeft(prev => {
        if (prev <= 1) {
          generateNewPairingSession();
          return 300;
        }
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const formatTime = (seconds: number) => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s < 10 ? "0" : ""}${s}`;
  };

  return (
    <div className="space-y-6 max-w-2xl mx-auto">
      <div className="text-center space-y-2">
        <h3 className="text-2xl font-bold text-white">Pair Your Mobile Device</h3>
        <p className="text-sm text-gray-400">
          Open the OpenTapUnlock app on your iPhone or Android and scan this secure QR code.
        </p>
      </div>

      <div className="glass-card p-8 rounded-3xl flex flex-col items-center justify-center space-y-6 border border-mint/20 relative overflow-hidden">
        <div className="absolute top-4 right-4 px-3 py-1 bg-surface rounded-full text-xs font-mono text-gray-400 border border-gray-800">
          Expires in: <span className="text-mint font-bold">{formatTime(timeLeft)}</span>
        </div>

        {/* QR Code Container */}
        <div className="p-6 bg-white rounded-2xl shadow-2xl">
          {pairingUri ? (
            <QRCodeSVG value={pairingUri} size={220} level="H" includeMargin={false} />
          ) : (
            <div className="w-[220px] h-[220px] bg-gray-200 animate-pulse rounded-xl" />
          )}
        </div>

        {/* Verification PIN Display */}
        <div className="text-center space-y-1">
          <p className="text-xs text-gray-400 font-semibold uppercase tracking-wider">Out-Of-Band Verification PIN</p>
          <div className="font-mono text-2xl font-bold tracking-widest text-mint px-6 py-2 bg-background rounded-xl border border-gray-800">
            {pin.slice(0, 3)} - {pin.slice(3, 6)}
          </div>
          <p className="text-[11px] text-gray-500 max-w-xs mx-auto pt-1">
            Confirm this PIN matches the code displayed on your phone after scanning.
          </p>
        </div>

        <button
          onClick={generateNewPairingSession}
          className="px-4 py-2 bg-surface text-gray-300 hover:text-white border border-gray-700 hover:border-gray-600 rounded-xl text-xs font-semibold transition flex items-center gap-2"
        >
          <RefreshCw className="w-3.5 h-3.5" />
          Generate New QR Code
        </button>
      </div>

      <div className="glass-card p-6 rounded-2xl flex items-start gap-4 border border-gray-800">
        <Shield className="w-6 h-6 text-mint shrink-0 mt-0.5" />
        <div className="space-y-1 text-xs text-gray-400">
          <p className="font-semibold text-white">How does Zero-Trust QR Pairing work?</p>
          <p className="leading-relaxed">
            Scanning this QR code performs an offline Diffie-Hellman / Ed25519 public key exchange. Your desktop private key never leaves this machine, and your phone private key never leaves your Secure Enclave.
          </p>
        </div>
      </div>
    </div>
  );
}
