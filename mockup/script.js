/**
 * Rcarve UI Mockup v2 - Professional Layout
 * 
 * Interactions:
 * - Menu bar dropdowns
 * - Context-sensitive inspector
 * - Tree view expand/collapse
 * - Tool/View selection
 * - Modal dialogs
 * - Panel collapse
 */

document.addEventListener('DOMContentLoaded', () => {
    initMenus();
    initToolbar();
    initTreeView();
    initInspector();
    initModals();
    initPanels();
    initCanvas();
    initKeyboardShortcuts();
});

/* --------------------------------------------------------------------------
   Menu Bar Dropdowns
   -------------------------------------------------------------------------- */
function initMenus() {
    const dropdowns = document.querySelectorAll('.menu-dropdown');
    
    dropdowns.forEach(dropdown => {
        const trigger = dropdown.querySelector('.menu-trigger');
        
        trigger.addEventListener('click', (e) => {
            e.stopPropagation();
            
            // Close other dropdowns
            dropdowns.forEach(d => {
                if (d !== dropdown) d.classList.remove('open');
            });
            
            dropdown.classList.toggle('open');
        });
        
        // Handle menu item clicks
        const items = dropdown.querySelectorAll('.menu-item');
        items.forEach(item => {
            item.addEventListener('click', () => {
                const action = item.dataset.action;
                if (action) handleMenuAction(action);
                dropdown.classList.remove('open');
            });
        });
    });
    
    // Close dropdowns when clicking outside
    document.addEventListener('click', () => {
        dropdowns.forEach(d => d.classList.remove('open'));
    });
    
    // Close on escape
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            dropdowns.forEach(d => d.classList.remove('open'));
        }
    });
}

function handleMenuAction(action) {
    switch (action) {
        case 'add-profile':
        case 'add-pocket':
        case 'add-vcarve':
            openModal('operationModal');
            // Update the operation type segmented control
            const type = action.replace('add-', '');
            selectOperationType(type);
            break;
        case 'generate':
            handleGenerate();
            break;
        case 'open-tools':
            openModal('toolLibraryModal');
            break;
    }
}

function selectOperationType(type) {
    const modal = document.getElementById('operationModal');
    if (!modal) return;
    
    const segments = modal.querySelectorAll('.segmented-control.large .segment');
    segments.forEach(seg => {
        seg.classList.remove('active');
        if (seg.textContent.toLowerCase().includes(type)) {
            seg.classList.add('active');
        }
    });
}

/* --------------------------------------------------------------------------
   Toolbar
   -------------------------------------------------------------------------- */
function initToolbar() {
    // Tool selection
    const toolBtns = document.querySelectorAll('.tool-btn[data-tool]');
    toolBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            toolBtns.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            const tool = btn.dataset.tool;
            const canvas = document.querySelector('.canvas-container');
            if (canvas) {
                canvas.style.cursor = tool === 'pan' ? 'grab' : 'crosshair';
            }
        });
    });
    
    // View toggle (2D/3D)
    const viewBtns = document.querySelectorAll('.view-btn');
    viewBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            viewBtns.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            // In real app, would switch canvas rendering mode
        });
    });
    
    // Zoom controls
    let zoom = 100;
    const zoomIndicator = document.querySelector('.zoom-indicator');
    
    const zoomIn = document.querySelector('.tool-btn[title*="Zoom In"]');
    const zoomOut = document.querySelector('.tool-btn[title*="Zoom Out"]');
    const zoomFit = document.querySelector('.tool-btn[title*="Fit"]');
    
    if (zoomIn) {
        zoomIn.addEventListener('click', () => {
            zoom = Math.min(zoom + 25, 400);
            updateZoom(zoom, zoomIndicator);
        });
    }
    
    if (zoomOut) {
        zoomOut.addEventListener('click', () => {
            zoom = Math.max(zoom - 25, 25);
            updateZoom(zoom, zoomIndicator);
        });
    }
    
    if (zoomFit) {
        zoomFit.addEventListener('click', () => {
            zoom = 100;
            updateZoom(zoom, zoomIndicator);
        });
    }
    
    // Action buttons (add operations)
    document.querySelectorAll('.action-btn[data-action]').forEach(btn => {
        btn.addEventListener('click', () => {
            handleMenuAction(btn.dataset.action);
        });
    });
    
    // Generate button
    const generateBtn = document.getElementById('generateBtn');
    if (generateBtn) {
        generateBtn.addEventListener('click', handleGenerate);
    }
}

function updateZoom(level, indicator) {
    if (indicator) indicator.textContent = `${level}%`;
    
    const svg = document.querySelector('.canvas-svg');
    if (svg) {
        const scale = level / 100;
        svg.style.transform = `translate(-50%, -50%) scale(${scale})`;
    }
}

function handleGenerate() {
    const btn = document.getElementById('generateBtn');
    if (!btn || btn.classList.contains('generating')) return;
    
    btn.classList.add('generating');
    btn.innerHTML = `
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" class="spin">
            <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="2" stroke-dasharray="25" stroke-dashoffset="5"/>
        </svg>
        Generating...
    `;
    
    setTimeout(() => {
        btn.classList.remove('generating');
        btn.innerHTML = `
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                <path d="M2 7l3.5 3.5L12 4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            Generate
        `;
        
        // Update dirty status indicators
        document.querySelectorAll('.tree-item-status.dirty').forEach(dot => {
            dot.classList.remove('dirty');
            dot.classList.add('ready');
        });
        
        document.querySelectorAll('.op-status-badge.dirty').forEach(badge => {
            badge.classList.remove('dirty');
            badge.classList.add('ready');
            badge.textContent = 'Ready';
        });
        
        // Update status bar
        const statusItem = document.querySelector('.status-left .status-item');
        if (statusItem) {
            statusItem.innerHTML = '<span class="status-dot ready"></span> 3 of 3 ready';
        }
    }, 1500);
}

/* --------------------------------------------------------------------------
   Tree View (Project Browser)
   -------------------------------------------------------------------------- */
function initTreeView() {
    // Expand all sections by default
    document.querySelectorAll('.tree-section').forEach(section => {
        section.classList.add('expanded');
    });
    
    // Toggle sections
    document.querySelectorAll('.tree-header').forEach(header => {
        header.addEventListener('click', () => {
            const section = header.closest('.tree-section');
            section.classList.toggle('expanded');
        });
    });
    
    // Tree item selection
    document.querySelectorAll('.tree-item').forEach(item => {
        item.addEventListener('click', (e) => {
            e.stopPropagation();
            
            // Deselect all
            document.querySelectorAll('.tree-item').forEach(i => i.classList.remove('selected'));
            
            // Select clicked
            item.classList.add('selected');
            
            // Update inspector
            const selectType = item.dataset.select;
            updateInspector(selectType);
            
            // Update status bar
            updateStatusBarSelection(selectType);
        });
    });
}

/* --------------------------------------------------------------------------
   Context-Sensitive Inspector
   -------------------------------------------------------------------------- */
function initInspector() {
    // Initial state - show stock inspector (first item selected)
    showInspectorSection('stock');
    
    // Segmented control interactions in inspector
    document.querySelectorAll('.inspector-section .segmented-control').forEach(control => {
        const segments = control.querySelectorAll('.segment');
        segments.forEach(seg => {
            seg.addEventListener('click', () => {
                segments.forEach(s => s.classList.remove('active'));
                seg.classList.add('active');
            });
        });
    });
}

function updateInspector(selectType) {
    if (selectType === 'stock') {
        showInspectorSection('stock');
    } else if (selectType?.startsWith('import-')) {
        showInspectorSection('import');
    } else if (selectType?.startsWith('op-')) {
        showInspectorSection('operation');
    } else {
        showInspectorSection('empty');
    }
}

function showInspectorSection(type) {
    const sections = document.querySelectorAll('.inspector-section');
    sections.forEach(section => {
        section.classList.add('hidden');
    });
    
    const target = document.getElementById(`inspector-${type}`);
    if (target) {
        target.classList.remove('hidden');
    }
}

function updateStatusBarSelection(selectType) {
    const statusItem = document.querySelector('.status-left');
    if (!statusItem) return;
    
    let label = 'None';
    if (selectType === 'stock') {
        label = 'Stock';
    } else if (selectType?.startsWith('import-')) {
        label = 'logo.svg';
    } else if (selectType?.startsWith('op-')) {
        const opNames = ['Profile Cut', 'Pocket Clear', 'V-Carve Logo'];
        const idx = parseInt(selectType.split('-')[1]) || 0;
        label = opNames[idx] || 'Operation';
    }
    
    statusItem.innerHTML = `
        <span class="status-item">
            <span class="status-dot ready"></span>
            2 of 3 ready
        </span>
        <span class="status-divider">|</span>
        <span class="status-item">Selection: ${label}</span>
    `;
}

/* --------------------------------------------------------------------------
   Modals
   -------------------------------------------------------------------------- */
function initModals() {
    // Close buttons
    document.querySelectorAll('[data-close-modal]').forEach(btn => {
        btn.addEventListener('click', () => {
            const modal = btn.closest('.modal-overlay');
            if (modal) closeModal(modal);
        });
    });
    
    // Click outside to close
    document.querySelectorAll('.modal-overlay').forEach(overlay => {
        overlay.addEventListener('click', (e) => {
            if (e.target === overlay) {
                closeModal(overlay);
            }
        });
    });
    
    // Segmented controls in modals
    document.querySelectorAll('.modal .segmented-control').forEach(control => {
        const segments = control.querySelectorAll('.segment');
        segments.forEach(seg => {
            seg.addEventListener('click', () => {
                segments.forEach(s => s.classList.remove('active'));
                seg.classList.add('active');
            });
        });
    });
}

function openModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) {
        modal.classList.add('active');
        document.body.style.overflow = 'hidden';
        
        // Focus first input
        setTimeout(() => {
            const input = modal.querySelector('input:not([type="hidden"]), select');
            if (input) input.focus();
        }, 100);
    }
}

function closeModal(modal) {
    modal.classList.remove('active');
    document.body.style.overflow = '';
}

/* --------------------------------------------------------------------------
   Panel Collapse
   -------------------------------------------------------------------------- */
function initPanels() {
    document.querySelectorAll('.panel-collapse').forEach(btn => {
        btn.addEventListener('click', () => {
            const panelSide = btn.dataset.panel;
            const panel = btn.closest('.panel');
            
            if (panel) {
                panel.classList.toggle('collapsed');
                
                // Flip the arrow
                const svg = btn.querySelector('svg');
                if (svg) {
                    if (panel.classList.contains('collapsed')) {
                        svg.style.transform = panelSide === 'left' ? 'rotate(180deg)' : 'rotate(180deg)';
                    } else {
                        svg.style.transform = '';
                    }
                }
            }
        });
    });
}

/* --------------------------------------------------------------------------
   Canvas Interactions
   -------------------------------------------------------------------------- */
function initCanvas() {
    const canvas = document.querySelector('.canvas-container');
    if (!canvas) return;
    
    let isPanning = false;
    let startX, startY;
    let translateX = 0, translateY = 0;
    
    // Mouse wheel zoom
    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        const indicator = document.querySelector('.zoom-indicator');
        let zoom = parseInt(indicator?.textContent) || 100;
        
        const delta = e.deltaY > 0 ? -10 : 10;
        zoom = Math.max(25, Math.min(400, zoom + delta));
        
        updateZoom(zoom, indicator);
    });
    
    // Pan with middle mouse or when pan tool is active
    canvas.addEventListener('mousedown', (e) => {
        const panBtn = document.querySelector('.tool-btn[data-tool="pan"]');
        if (e.button === 1 || panBtn?.classList.contains('active')) {
            isPanning = true;
            startX = e.clientX - translateX;
            startY = e.clientY - translateY;
            canvas.style.cursor = 'grabbing';
        }
    });
    
    canvas.addEventListener('mousemove', (e) => {
        // Update coordinates display
        const rect = canvas.getBoundingClientRect();
        const x = ((e.clientX - rect.left) / rect.width * 200).toFixed(2);
        const y = (150 - (e.clientY - rect.top) / rect.height * 150).toFixed(2);
        
        const coordDisplay = document.querySelector('.status-right .status-item:first-child');
        if (coordDisplay) {
            coordDisplay.innerHTML = `
                <span class="coord-label">X:</span>
                <span class="coord-value mono">${x}</span>
                <span class="coord-label">Y:</span>
                <span class="coord-value mono">${y}</span>
            `;
        }
        
        // Handle panning
        if (isPanning) {
            translateX = e.clientX - startX;
            translateY = e.clientY - startY;
            
            const svg = document.querySelector('.canvas-svg');
            const label = document.querySelector('.canvas-label');
            const grid = document.querySelector('.canvas-grid');
            
            if (svg) {
                svg.style.left = `calc(50% + ${translateX}px)`;
                svg.style.top = `calc(50% + ${translateY}px)`;
            }
            if (label) {
                label.style.left = `calc(50% + ${translateX}px)`;
            }
            if (grid) {
                grid.style.backgroundPosition = `calc(50% + ${translateX}px) calc(50% + ${translateY}px)`;
            }
        }
    });
    
    canvas.addEventListener('mouseup', () => {
        if (isPanning) {
            isPanning = false;
            const panBtn = document.querySelector('.tool-btn[data-tool="pan"]');
            canvas.style.cursor = panBtn?.classList.contains('active') ? 'grab' : 'crosshair';
        }
    });
    
    canvas.addEventListener('mouseleave', () => {
        isPanning = false;
    });
}

/* --------------------------------------------------------------------------
   Keyboard Shortcuts
   -------------------------------------------------------------------------- */
function initKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
        // Skip if in input or modal is open
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
        if (document.querySelector('.modal-overlay.active')) {
            if (e.key === 'Escape') {
                const modal = document.querySelector('.modal-overlay.active');
                if (modal) closeModal(modal);
            }
            return;
        }
        
        const isMeta = e.metaKey || e.ctrlKey;
        
        // Tool shortcuts
        if (e.key === 'v') {
            const selectBtn = document.querySelector('.tool-btn[data-tool="select"]');
            if (selectBtn) selectBtn.click();
        }
        if (e.key === 'h') {
            const panBtn = document.querySelector('.tool-btn[data-tool="pan"]');
            if (panBtn) panBtn.click();
        }
        
        // Generate
        if (isMeta && e.key === 'g') {
            e.preventDefault();
            handleGenerate();
        }
        
        // Tool library
        if (isMeta && e.key === 't') {
            e.preventDefault();
            openModal('toolLibraryModal');
        }
        
        // Zoom shortcuts
        if (e.key === '0' && !isMeta) {
            const fitBtn = document.querySelector('.tool-btn[title*="Fit"]');
            if (fitBtn) fitBtn.click();
        }
        if (e.key === '=' || e.key === '+') {
            const zoomIn = document.querySelector('.tool-btn[title*="Zoom In"]');
            if (zoomIn) zoomIn.click();
        }
        if (e.key === '-') {
            const zoomOut = document.querySelector('.tool-btn[title*="Zoom Out"]');
            if (zoomOut) zoomOut.click();
        }
    });
}

/* --------------------------------------------------------------------------
   Utility: Add spin animation style
   -------------------------------------------------------------------------- */
const style = document.createElement('style');
style.textContent = `
    @keyframes spin {
        from { transform: rotate(0deg); }
        to { transform: rotate(360deg); }
    }
    .spin { animation: spin 0.8s linear infinite; }
`;
document.head.appendChild(style);

console.log('Rcarve UI Mockup v2 loaded.');
