"use client";

import React, { useState, useEffect, useCallback } from "react";
import {
  Activity,
  Scissors,
  Mic,
  Sparkles,
  Eraser,
  Clapperboard,
  AudioLines,
  Film,
  RefreshCw,
  CheckCircle2,
  AlertCircle,
  Loader2,
} from "lucide-react";
import { motion } from "framer-motion";

interface NodeInfo {
  name: string;
  url: string;
  status: "healthy" | "degraded" | "unreachable";
  latency?: number;
  description: string;
  icon: React.ElementType;
  gpu?: boolean;
}

const API_BASE =
  process.env.NEXT_PUBLIC_API_URL || "https://orchestrator.p.zacharie.org";

const NODES: Omit<NodeInfo, "status" | "latency">[] = [
  {
    name: "orchestrator",
    url: `${API_BASE}/health`,
    description: "Core pipeline orchestration service",
    icon: Activity,
  },
  {
    name: "Extractor",
    url: `${API_BASE}/health`,
    description: "Frame & audio extraction engine",
    icon: Scissors,
  },
  {
    name: "Voice Extractor",
    url: `${API_BASE}/health`,
    description: "Speech-to-text transcription (Whisper)",
    icon: Mic,
    gpu: true,
  },
  {
    name: "Diffusion Engine",
    url: `${API_BASE}/health`,
    description: "SDXL Turbo image generation",
    icon: Sparkles,
    gpu: true,
  },
  {
    name: "Dewatermark",
    url: `${API_BASE}/health`,
    description: "Watermark removal GPU pipeline",
    icon: Eraser,
    gpu: true,
  },
  {
    name: "Video Composer",
    url: `${API_BASE}/health`,
    description: "Final video assembly & mux",
    icon: Clapperboard,
    gpu: true,
  },
  {
    name: "Voice Cloner",
    url: `${API_BASE}/health`,
    description: "TTS voice cloning (GPT-SoVITS)",
    icon: AudioLines,
    gpu: true,
  },
  {
    name: "Video Generator",
    url: `${API_BASE}/health`,
    description: "SVD AI video generation engine",
    icon: Film,
    gpu: true,
  },
];

export default function NodesPage() {
  const [nodes, setNodes] = useState<NodeInfo[]>(
    NODES.map((n) => ({ ...n, status: "unreachable" as const }))
  );
  const [loading, setLoading] = useState(true);
  const [lastChecked, setLastChecked] = useState<Date | null>(null);

  const checkNodes = useCallback(async () => {
    setLoading(true);
    const results = await Promise.all(
      NODES.map(async (node) => {
        const start = Date.now();
        try {
          const res = await fetch(node.url, {
            signal: AbortSignal.timeout(5000),
          });
          const latency = Date.now() - start;
          return {
            ...node,
            status: (res.ok ? "healthy" : "degraded") as NodeInfo["status"],
            latency,
          };
        } catch {
          return { ...node, status: "unreachable" as const };
        }
      })
    );
    setNodes(results);
    setLastChecked(new Date());
    setLoading(false);
  }, []);

  useEffect(() => {
    checkNodes();
  }, [checkNodes]);

  const healthy = nodes.filter((n) => n.status === "healthy").length;
  const degraded = nodes.filter((n) => n.status === "degraded").length;
  const unreachable = nodes.filter((n) => n.status === "unreachable").length;

  const statusColor = {
    healthy: "text-emerald-400 border-emerald-700/40 bg-emerald-950/20",
    degraded: "text-amber-400 border-amber-700/40 bg-amber-950/20",
    unreachable: "text-red-400 border-red-700/40 bg-red-950/20",
  };

  return (
    <div className="space-y-8 pb-10">
      <div className="flex items-center justify-between flex-wrap gap-4">
        <div>
          <h2 className="text-4xl font-extrabold tracking-tight">
            Node Status
          </h2>
          <p className="text-slate-400 font-medium mt-1">
            Santé des services de la pipeline
            {lastChecked && (
              <span className="ml-2 text-slate-600 text-xs">
                — dernière vérification {lastChecked.toLocaleTimeString()}
              </span>
            )}
          </p>
        </div>
        <button
          onClick={checkNodes}
          disabled={loading}
          className="p-2.5 glass rounded-xl hover:bg-white/10 transition-colors"
        >
          <RefreshCw className={`w-4 h-4 ${loading ? "animate-spin" : ""}`} />
        </button>
      </div>

      {/* Summary */}
      <div className="grid grid-cols-3 gap-4">
        <div className="glass rounded-2xl p-5 border border-emerald-700/30">
          <p className="text-2xl font-bold text-emerald-400">{healthy}</p>
          <p className="text-sm text-slate-400 mt-1">Healthy</p>
        </div>
        <div className="glass rounded-2xl p-5 border border-amber-700/30">
          <p className="text-2xl font-bold text-amber-400">{degraded}</p>
          <p className="text-sm text-slate-400 mt-1">Degraded</p>
        </div>
        <div className="glass rounded-2xl p-5 border border-red-700/30">
          <p className="text-2xl font-bold text-red-400">{unreachable}</p>
          <p className="text-sm text-slate-400 mt-1">Unreachable</p>
        </div>
      </div>

      {/* Node cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {nodes.map((node, i) => {
          const Icon = node.icon;
          return (
            <motion.div
              key={node.name}
              initial={{ opacity: 0, y: 16 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.05 }}
              className={`glass rounded-2xl p-5 border ${statusColor[node.status]} relative overflow-hidden`}
            >
              <div className="flex items-start gap-4">
                <div className="p-3 rounded-xl bg-white/5">
                  <Icon className="w-5 h-5 text-slate-400" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <p className="font-bold">{node.name}</p>
                    {node.gpu && (
                      <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-purple-950 border border-purple-700/40 text-purple-300 uppercase tracking-wider">
                        GPU
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-slate-500 mt-0.5">
                    {node.description}
                  </p>
                </div>
                <div className="flex flex-col items-end gap-1 shrink-0">
                  {loading ? (
                    <Loader2 className="w-4 h-4 animate-spin text-slate-600" />
                  ) : node.status === "healthy" ? (
                    <CheckCircle2 className="w-5 h-5 text-emerald-400" />
                  ) : (
                    <AlertCircle className="w-5 h-5 text-red-400" />
                  )}
                  {node.latency !== undefined && (
                    <span className="text-[10px] text-slate-500 font-mono">
                      {node.latency}ms
                    </span>
                  )}
                </div>
              </div>

              {/* Status bar */}
              <div className="mt-4 h-0.5 w-full rounded-full bg-white/5 overflow-hidden">
                {!loading && node.status === "healthy" && (
                  <div className="h-full bg-emerald-500 w-full transition-all duration-1000" />
                )}
              </div>
            </motion.div>
          );
        })}
      </div>
    </div>
  );
}
