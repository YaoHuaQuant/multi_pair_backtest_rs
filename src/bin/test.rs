use plotters::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建一个绘图区域，输出为 PNG 文件，尺寸为 600x400 像素
    let root_area = BitMapBackend::new("line_chart.png", (600, 400)).into_drawing_area();
    root_area.fill(&WHITE)?;

    // 构建图表上下文，设置标题和坐标轴范围
    let mut chart = ChartBuilder::on(&root_area)
        .caption("折线图示例", ("sans-serif", 40))
        .margin(20)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0..10, 0..100)?;

    // 绘制坐标网格
    chart.configure_mesh().draw()?;

    // 准备数据点，例如 y = x^2
    let data: Vec<(i32, i32)> = (0..=10).map(|x| (x, x * x)).collect();

    // 绘制折线图
    chart.draw_series(LineSeries::new(data, &BLUE))?;

    Ok(())
}
