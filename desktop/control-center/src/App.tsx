import React, { useState } from "react";
import { Shield, Smartphone, QrCode, FileText, Lock, Zap } from "lucide-react";
import Dashboard from "./components/Dashboard";
import PairedDevices from "./components/PairedDevices";
import PairingQrModal from "./components/PairingQrModal";
import AuditLogs from "./components/AuditLogs";

export default function App() {
  const [activeTab, setActiveTab] = useState<"dashboard" | "devices" | "pair" | "logs">("dashboard");

  return (
    <div className="flex h-screen w-screen bg-background text-gray-100 font-sans overflow-hidden">
      {/* Sidebar Navigation */}
      <aside className="w-64 bg-surface border-r border-gray-800 flex flex-col justify-between">
        <div>
          <div className="p-6 flex items-center gap-3 border-b border-gray-800">
            <div className="p-2 bg-mint/10 rounded-xl border border-mint/20 text-mint">
              <Shield className="w-6 h-6 animate-pulse" />
            </div>
            <div>
              <h1 className="font-bold text-lg tracking-tight text-white">OpenTap</h1>
              <p className="text-xs text-mint font-medium">Control Center v1.0</p>
            </div>
          </div>

          <nav className="p-4 space-y-1">
            <button
              onClick={() => setActiveTab("dashboard")}
              className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl font-medium text-sm transition-all ${
                activeTab === "dashboard"
                  ? "bg-mint/15 text-mint border border-mint/25 glow-mint"
                  : "text-gray-400 hover:text-white hover:bg-surface-light"
              }`}
            >
              <Zap className="w-5 h-5" />
              System Status
            </button>

            <button
              onClick={() => setActiveTab("devices")}
              className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl font-medium text-sm transition-all ${
                activeTab === "devices"
                  ? "bg-mint/15 text-mint border border-mint/25 glow-mint"
                  : "text-gray-400 hover:text-white hover:bg-surface-light"
              }`}
            >
              <Smartphone className="w-5 h-5" />
              Paired Phones
            </button>

            <button
              onClick={() => setActiveTab("pair")}
              className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl font-medium text-sm transition-all ${
                activeTab === "pair"
                  ? "bg-mint/15 text-mint border border-mint/25 glow-mint"
                  : "text-gray-400 hover:text-white hover:bg-surface-light"
              }`}
            >
              <QrCode className="w-5 h-5" />
              Pair New Phone
            </button>

            <button
              onClick={() => setActiveTab("logs")}
              className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl font-medium text-sm transition-all ${
                activeTab === "logs"
                  ? "bg-mint/15 text-mint border border-mint/25 glow-mint"
                  : "text-gray-400 hover:text-white hover:bg-surface-light"
              }`}
            >
              <FileText className="w-5 h-5" />
              Security Audit Logs
            </button>
          </nav>
        </div>

        {/* Zero Battery Drain Badge in Sidebar */}
        <div className="p-4 m-4 rounded-xl bg-surface-light/50 border border-gray-800 text-xs text-gray-400">
          <div className="flex items-center gap-2 text-mint font-semibold mb-1">
            <span className="w-2 h-2 rounded-full bg-mint animate-ping" />
            0.00% CPU Drain
          </div>
          <p className="leading-relaxed">
            Native OS WebView & Rust async epoll active. Zero battery impact.
          </p>
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col h-full bg-background overflow-y-auto">
        <header className="h-16 border-b border-gray-800 px-8 flex items-center justify-between bg-surface/40 backdrop-blur-md sticky top-0 z-10">
          <h2 className="text-lg font-semibold text-white capitalize">
            {activeTab === "devices" ? "Authorized Mobile Devices" : activeTab === "pair" ? "Pair New Device" : activeTab}
          </h2>
          <div className="flex items-center gap-3">
            <span className="inline-flex items-center gap-2 px-3 py-1 rounded-full text-xs font-semibold bg-green-500/10 text-green-400 border border-green-500/20">
              <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
              Daemon Online
            </span>
          </div>
        </header>

        <div className="p-8 max-w-5xl">
          {activeTab === "dashboard" && <Dashboard onNavigate={setActiveTab} />}
          {activeTab === "devices" && <PairedDevices />}
          {activeTab === "pair" && <PairingQrModal />}
          {activeTab === "logs" && <AuditLogs />}
        </div>
      </main>
    </div>
  );
}
