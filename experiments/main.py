import matplotlib.pyplot as plt
import numpy as np
from shapely.geometry import Polygon, LineString
from shapely.ops import voronoi_diagram

def generate_hybrid_visual():
    fig, ax = plt.subplots(figsize=(10, 6))

    # Define a "Wide V" shape that exceeds max depth
    # It starts narrow at x=0, and gets wide at x=10
    polygon_points = [
        (0, 0),      # Tip
        (10, 4),     # Top Right (Wide)
        (10, -4),    # Bottom Right (Wide)
        (0, 0)       # Close loop
    ]
    
    poly = Polygon(polygon_points)
    
    # Max Radius (corresponds to Max Depth)
    MAX_RADIUS = 1.5
    
    # 1. Generate Offset (The "Split" paths)
    # This represents the path the tool takes when at max depth
    offset_poly = poly.buffer(-MAX_RADIUS)
    
    # 2. Generate Medial Axis (Simulated via centerline logic for this simple shape)
    # In a real Voronoi, this would be calculated automatically.
    # The centerline is simply y=0 from x=0 to x=10.
    # We only keep it where width < 2 * MAX_RADIUS.
    # Width at x is (4/10)*x * 2. 
    # 0.8x < 3.0  => x < 3.75
    
    cutoff_x = 3.75
    medial_x = np.linspace(0, cutoff_x, 100)
    medial_y = np.zeros_like(medial_x)

    # Plotting
    x, y = poly.exterior.xy
    ax.plot(x, y, 'k-', linewidth=2, label='Shape Boundary')
    
    # Plot the "Split" (Offset) Paths
    if not offset_poly.is_empty:
        # Handle MultiPolygon if needed, usually it's a Polygon or MultiPolygon
        if offset_poly.geom_type == 'Polygon':
            ox, oy = offset_poly.exterior.xy
            ax.plot(ox, oy, 'b-', linewidth=3, label=f'Offset Path (Max Depth={MAX_RADIUS})')
        else:
            for geom in offset_poly.geoms:
                ox, oy = geom.exterior.xy
                ax.plot(ox, oy, 'b-', linewidth=3)

    # Plot the Medial Axis (V-Carve)
    ax.plot(medial_x, medial_y, 'g-', linewidth=3, label='Medial Axis (Variable Depth)')
    
    # Add annotations
    ax.annotate('Start V-Carve\n(Depth = 0)', xy=(0, 0), xytext=(-2, 0),
                arrowprops=dict(facecolor='black', shrink=0.05))
    
    ax.annotate('The "Bifurcation"\n(Depth Reached Max)', xy=(cutoff_x, 0), xytext=(cutoff_x, -2.5),
                arrowprops=dict(facecolor='black', shrink=0.05), ha='center')

    # Draw the circle at the split point to visualize constraints
    circle = plt.Circle((cutoff_x, 0), MAX_RADIUS, color='red', fill=False, linestyle='--', label='Max Bit Size')
    ax.add_patch(circle)

    ax.set_title("Hybrid Strategy: Medial Axis -> Parallel Offset", fontsize=14)
    ax.set_aspect('equal')
    ax.legend()
    ax.grid(True, alpha=0.3)
    
    plt.savefig('hybrid_vcarve.png')

generate_hybrid_visual()
