# Rcarve UI Mockup

A static HTML/CSS/JS mockup serving as a north-star design reference for the Rcarve CNC CAM application UI.

## Usage

Open `index.html` in any modern web browser. No build step or server required.

---

## Application Functionality Reference

### 1. Project Management

Rcarve uses a project-based workflow where all settings, imports, and operations are saved to a `.rcproj` file.

**Features:**
- **New Project**: Create a fresh project file (Cmd/Ctrl + N)
- **Open Project**: Load an existing project (Cmd/Ctrl + O)
- **Auto-save**: Changes are automatically persisted
- **Project Name**: Displayed in the window title and sidebar header

---

### 2. Stock Definition (Workpiece)

The stock represents the physical material being machined.

**Parameters:**
| Field | Description | Unit |
|-------|-------------|------|
| Width | X-axis dimension | mm |
| Height | Y-axis dimension | mm |
| Thickness | Z-axis dimension (material depth) | mm |
| Material | Optional label (e.g., "MDF", "Walnut") | text |
| Origin | Coordinate system reference point | (x, y, z) |

**UI:**
- Displayed as a card in the Stock tab
- "Edit Stock" button opens a modal dialog
- Stock bounds visualized as a rectangle on the 2D canvas
- Stock rendered as a 3D box in the 3D view

---

### 3. SVG Import System

Vector artwork is imported from SVG files and converted to curves for machining.

**Features:**
- Import multiple SVG files per project (Cmd/Ctrl + I)
- Each import is tracked with:
  - Source file path
  - Display label
  - Extracted curve IDs
  - Extracted shape IDs
- Select imports by clicking in the sidebar or canvas
- Delete imports when no longer needed

**Transform Manipulation:**
When an import is selected, gizmos appear for direct manipulation:
- **Translate**: Drag the body to reposition
- **Rotate**: Drag the rotation handle (circle above bounding box)
- **Scale**: Drag corner handles (squares at bounding box corners)
- Transforms are applied uniformly; scaling preserves aspect ratio

---

### 4. Tool Library

Tools are defined globally and persist across all projects.

**Tool Types:**

| Type | Description | Key Parameter |
|------|-------------|---------------|
| Endmill | Flat-bottomed cylindrical cutter | Diameter |
| V-Bit | Angled tip for engraving/chamfering | Included angle (°) |
| Ballnose | Hemispherical tip for 3D contouring | Diameter |

**Common Parameters:**
- **Name**: User-friendly identifier
- **Diameter**: Cutting width (mm)
- **Stepover**: Percentage of diameter for parallel passes (0.0–1.0)
- **Pass Depth**: Maximum Z cut per pass (mm)

**UI:**
- Tools tab shows the full library as cards
- Add, edit, and delete tools via buttons
- Tool selection in operation forms uses a dropdown picker

---

### 5. CAM Operations

Operations define how geometry is machined. Each operation targets specific curves and uses a tool from the library.

#### Profile Operation
Cut along the perimeter of curves.

| Setting | Options |
|---------|---------|
| Cut Side | Outside, Inside, On-Line |
| Target Depth | Final Z position (mm) |
| Tool | Select from library |

#### Pocket Operation
Clear the interior of enclosed regions.

| Setting | Description |
|---------|-------------|
| Target Depth | Final Z position (mm) |
| Tool | Uses stepover for parallel passes |

#### V-Carve Operation
Variable-depth carving following the medial axis (Voronoi-based).

| Setting | Description |
|---------|-------------|
| Max Depth | Optional limit for flat-bottom hybrid carving |
| Tool | Must be a V-Bit |
| Clearance Tool | Optional second tool for large flat areas |

**Operation Status:**
- **Dirty**: Needs regeneration (operation modified since last generate)
- **Ready**: Toolpath generated successfully
- **Ready (warnings)**: Generated with non-critical issues
- **Invalid**: Cannot generate (missing data, invalid configuration)

---

### 6. Toolpath Generation

**Workflow:**
1. Define stock dimensions
2. Import SVG artwork
3. Create operations targeting curves
4. Click "Generate Toolpaths" to compute all dirty operations

**Features:**
- Per-operation visibility toggle (show/hide on canvas)
- Per-operation "Clear" button to remove generated toolpath
- Multi-pass depth handling (automatic based on tool pass depth)
- Color-coded toolpaths (each operation gets a unique color)
- Hover operation cards to highlight corresponding toolpath

---

### 7. 2D Canvas View

The primary workspace for positioning artwork and viewing toolpaths.

**Navigation:**
| Action | Control |
|--------|---------|
| Pan | Left-drag on background |
| Zoom | Scroll wheel |

**Visualizations:**
- Stock bounds (rectangle outline)
- Imported curves (gray when unselected, highlighted when selected)
- Toolpaths (colored polylines, one color per operation)
- Selection gizmos (bounding box, rotation handle, scale handles)

**Coordinate System:**
- Origin at bottom-left of stock (configurable)
- Y-axis points up
- Grid aligned to stock bounds

---

### 8. 3D Preview View

Visualize the machining setup in three dimensions.

**Navigation:**
| Action | Control |
|--------|---------|
| Orbit | Left-drag |
| Pan | Right-drag |
| Zoom | Scroll wheel |

**Options:**
- **Stock Rendering**: Toggle between solid and wireframe
- **Imported Curves**: Show/hide curves on stock surface
- Toolpaths rendered with Z-depth information

---

### 9. Debug Visualization

Developer tools for inspecting internal geometry.

**Polygon Outlines:**
Toggle to show the computed polygon boundaries for operations.

**V-Carve Debug Settings:**
Advanced visualization for V-carve algorithm internals:
- Pre-prune Voronoi edges (all computed edges)
- Post-prune Voronoi edges (kept after angle filtering)
- Pruned edges (removed by filtering)
- Crease paths (variable-depth medial axis paths)
- Pocket boundary paths (constant-depth offset boundaries)

---

## Design Philosophy

This mockup establishes a **dark industrial aesthetic** appropriate for precision machining software:

- Deep blue-gray tones evoke workshop/industrial environments
- Electric cyan accents for primary actions and highlights
- Orange accents for secondary/destructive actions
- High contrast for readability during focused work
- Monospace typography for technical values (dimensions, coordinates)
- Clean sans-serif for UI labels and headings
- Generous whitespace and clear visual hierarchy
- Subtle animations for state changes and feedback





