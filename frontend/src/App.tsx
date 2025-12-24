import { useState, useCallback, useMemo, useEffect } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { GraphCanvas } from './components/GraphCanvas';
import type { LayoutType } from './components/GraphCanvas';
import { Controls } from './components/Controls';
import type { DirectionType } from './components/Controls';
import { NodeDetails } from './components/NodeDetails';
import { Legend } from './components/Legend';
import { useHealth, useRepos, useGraph } from './hooks/useCodeIntel';
import type { ViewType, GraphNode, CodeGraph } from './types/graph';

// Create a client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60000,
      retry: 1,
    },
  },
});

function AppContent() {
  // State
  const [selectedRepo, setSelectedRepo] = useState<string>('');
  const [view, setView] = useState<ViewType>('call');
  const [depth, setDepth] = useState(2); // Start with smaller depth for faster initial load
  const [root, setRoot] = useState<string | undefined>();
  const [direction, setDirection] = useState<DirectionType>('both');
  const [maxNodes, setMaxNodes] = useState(100); // Limit nodes for readable graphs
  const [showMetrics, setShowMetrics] = useState(false); // Start without metrics for faster initial load
  const [showSecurity, setShowSecurity] = useState(false); // Load security on demand
  const [clustered, setClustered] = useState(false);
  const [layout, setLayout] = useState<LayoutType>('dagre'); // Use hierarchical by default
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);

  // Queries
  const { data: health, isLoading: healthLoading, error: healthError } = useHealth();
  const { data: repos } = useRepos();

  // Auto-select first repo when repos load
  useEffect(() => {
    if (repos && repos.length > 0 && !selectedRepo) {
      setSelectedRepo(repos[0]);
    }
  }, [repos, selectedRepo]);

  const {
    data: graphResponse,
    isLoading: graphLoading,
    error: graphError,
    refetch,
  } = useGraph(
    {
      repo: selectedRepo,
      view,
      depth,
      root,
      direction: view === 'call' ? direction : undefined,
      include_metrics: showMetrics,
      include_security: showSecurity,
      cluster_by: clustered ? 'file' : 'none',
    },
    !!selectedRepo
  );

  // Apply maxNodes limit to the graph (client-side filtering)
  const limitedGraph = useMemo((): CodeGraph | null => {
    if (!graphResponse?.graph) return null;

    const graph = graphResponse.graph;

    // If maxNodes is at max (500), don't limit
    if (maxNodes >= 500 || graph.nodes.length <= maxNodes) {
      return graph;
    }

    // Sort nodes by connectivity (most connected first) to keep important nodes
    const nodeConnections = new Map<string, number>();
    for (const node of graph.nodes) {
      nodeConnections.set(node.id, 0);
    }
    for (const edge of graph.edges) {
      nodeConnections.set(edge.source, (nodeConnections.get(edge.source) || 0) + 1);
      nodeConnections.set(edge.target, (nodeConnections.get(edge.target) || 0) + 1);
    }

    const sortedNodes = [...graph.nodes].sort((a, b) => {
      return (nodeConnections.get(b.id) || 0) - (nodeConnections.get(a.id) || 0);
    });

    // Take top N nodes
    const keptNodes = new Set(sortedNodes.slice(0, maxNodes).map(n => n.id));
    const limitedNodes = graph.nodes.filter(n => keptNodes.has(n.id));
    const limitedEdges = graph.edges.filter(
      e => keptNodes.has(e.source) && keptNodes.has(e.target)
    );

    return {
      ...graph,
      nodes: limitedNodes,
      edges: limitedEdges,
      metadata: {
        ...graph.metadata,
        node_count: limitedNodes.length,
        edge_count: limitedEdges.length,
      },
    };
  }, [graphResponse?.graph, maxNodes]);

  // Handlers
  const handleNodeSelect = useCallback((node: GraphNode | null) => {
    setSelectedNode(node);
  }, []);

  const handleNodeDoubleClick = useCallback((node: GraphNode) => {
    // Set this node as the root for focused exploration
    setRoot(node.id);
  }, []);

  const handleNavigate = useCallback((filePath: string, line: number) => {
    // In a real IDE integration, this would open the file
    console.log(`Navigate to: ${filePath}:${line}`);
    // For now, just alert
    alert(`Would open: ${filePath}:${line}`);
  }, []);

  // Connection status
  if (healthLoading) {
    return (
      <div className="h-screen flex items-center justify-center bg-slate-50 dark:bg-slate-950">
        <div className="text-center">
          <div className="relative w-12 h-12 mx-auto mb-4">
            <div className="absolute inset-0 rounded-full border-2 border-slate-200 dark:border-slate-800"></div>
            <div className="absolute inset-0 rounded-full border-2 border-blue-500 border-t-transparent animate-spin"></div>
          </div>
          <p className="text-slate-500 dark:text-slate-400 text-sm font-medium">Connecting to server...</p>
        </div>
      </div>
    );
  }

  if (healthError) {
    return (
      <div className="h-screen flex items-center justify-center bg-slate-50 dark:bg-slate-950">
        <div className="text-center max-w-md px-6">
          <div className="w-12 h-12 mx-auto mb-4 rounded-full bg-red-50 dark:bg-red-950 flex items-center justify-center">
            <svg className="w-6 h-6 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </div>
          <h2 className="text-lg font-semibold text-slate-900 dark:text-white mb-2">
            Connection Failed
          </h2>
          <p className="text-slate-500 dark:text-slate-400 text-sm mb-4">
            Could not connect to the code-intel server. Make sure it's running with the{' '}
            <code className="px-1.5 py-0.5 bg-slate-100 dark:bg-slate-800 rounded text-xs font-mono">--http</code> flag.
          </p>
          <code className="block bg-slate-900 dark:bg-slate-800 text-slate-100 p-4 rounded-lg text-xs text-left font-mono">
            ./code-intel --repos . --http --call-graph
          </code>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-slate-50 dark:bg-slate-950">
      {/* Header */}
      <header className="bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 px-5 py-3 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-blue-500 flex items-center justify-center">
              <svg className="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
            </div>
            <h1 className="text-base font-semibold text-slate-900 dark:text-white tracking-tight">
              code-intel
            </h1>
          </div>
          <span className="px-2 py-0.5 text-[10px] font-medium bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400 rounded-full uppercase tracking-wide">
            v{health?.version ?? '?'}
          </span>
        </div>
        <div className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
          <span className="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse"></span>
          <span className="font-medium">Connected</span>
        </div>
      </header>

      {/* Controls */}
      <Controls
        repos={repos ?? []}
        selectedRepo={selectedRepo}
        onRepoChange={setSelectedRepo}
        view={view}
        onViewChange={setView}
        depth={depth}
        onDepthChange={setDepth}
        root={root}
        onRootChange={setRoot}
        direction={direction}
        onDirectionChange={setDirection}
        maxNodes={maxNodes}
        onMaxNodesChange={setMaxNodes}
        showMetrics={showMetrics}
        onShowMetricsChange={setShowMetrics}
        showSecurity={showSecurity}
        onShowSecurityChange={setShowSecurity}
        clustered={clustered}
        onClusteredChange={setClustered}
        layout={layout}
        onLayoutChange={setLayout}
        loading={graphLoading}
        onRefresh={() => refetch()}
      />

      {/* Main content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Graph canvas */}
        <div className="flex-1 relative">
          {!selectedRepo ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-center">
                <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-slate-100 dark:bg-slate-800 flex items-center justify-center">
                  <svg
                    className="w-8 h-8 text-slate-400 dark:text-slate-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={1.5}
                      d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
                    />
                  </svg>
                </div>
                <p className="text-slate-500 dark:text-slate-400 text-sm">
                  Select a repository to visualize
                </p>
              </div>
            </div>
          ) : graphLoading ? (
            <div className="h-full flex items-center justify-center">
              <div className="relative w-8 h-8">
                <div className="absolute inset-0 rounded-full border-2 border-slate-200 dark:border-slate-800"></div>
                <div className="absolute inset-0 rounded-full border-2 border-blue-500 border-t-transparent animate-spin"></div>
              </div>
            </div>
          ) : graphError || graphResponse?.error ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-center px-6">
                <div className="w-12 h-12 mx-auto mb-3 rounded-full bg-red-50 dark:bg-red-950 flex items-center justify-center">
                  <svg className="w-5 h-5 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                  </svg>
                </div>
                <p className="text-slate-900 dark:text-white font-medium text-sm mb-1">Error loading graph</p>
                <p className="text-slate-500 dark:text-slate-400 text-xs max-w-xs">
                  {graphResponse?.error ?? String(graphError)}
                </p>
              </div>
            </div>
          ) : (
            <GraphCanvas
              graph={limitedGraph}
              onNodeSelect={handleNodeSelect}
              onNodeDoubleClick={handleNodeDoubleClick}
              layout={layout}
            />
          )}

          {/* Graph stats overlay */}
          {limitedGraph && (
            <div className="absolute bottom-4 left-4 bg-white/90 dark:bg-slate-900/90 backdrop-blur-sm rounded-lg shadow-sm border border-slate-200 dark:border-slate-800 px-3 py-2 text-xs text-slate-600 dark:text-slate-400">
              <span className="font-semibold text-slate-900 dark:text-white">{limitedGraph.metadata.node_count}</span> nodes
              <span className="mx-1.5 text-slate-300 dark:text-slate-700">·</span>
              <span className="font-semibold text-slate-900 dark:text-white">{limitedGraph.metadata.edge_count}</span> edges
              {graphResponse?.graph && graphResponse.graph.nodes.length > limitedGraph.nodes.length && (
                <span className="ml-2 text-amber-600 dark:text-amber-400">
                  (top {maxNodes} of {graphResponse.graph.nodes.length})
                </span>
              )}
            </div>
          )}
        </div>

        {/* Side panel */}
        <div className="w-80 bg-white dark:bg-slate-900 border-l border-slate-200 dark:border-slate-800 flex flex-col">
          <div className="border-b border-slate-200 dark:border-slate-800 px-4 py-3">
            <h2 className="text-sm font-semibold text-slate-900 dark:text-white">Details</h2>
          </div>
          <div className="flex-1 overflow-auto">
            <NodeDetails
              node={selectedNode}
              onClose={() => setSelectedNode(null)}
              onNavigate={handleNavigate}
            />
          </div>
        </div>
      </div>

      {/* Legend */}
      <Legend />
    </div>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AppContent />
    </QueryClientProvider>
  );
}

export default App;
