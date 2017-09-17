package xyz.clieb.bodygraph

import java.nio.file.{Path, Paths}
import java.time.{LocalDate, ZoneId}

import co.theasi.plotly._
import co.theasi.plotly.writer.{FileOptions, PlotFile}
import org.apache.poi.ss.usermodel.WorkbookFactory
import xyz.clieb.bodygraph.Closable._

import scala.util.{Failure, Success}

object Main {
  def main(args: Array[String]): Unit = {
    val main = new Main()
    val records = main.readFile(Paths.get("C:", "Users", "Chris", "My Tresors", "Official Documents", "frohman", "Body Tracker.xlsx"))
    val outFile = main.drawWeightGraph(records)
    println(outFile)
  }
}

class Main {
  def readFile(path: Path): Seq[Record] = {
    println(s"Reading data from file: ${path.toString}")

    closable(WorkbookFactory.create(path.toFile)) { workbook =>
      val sheet = workbook.getSheetAt(0)
      (sheet.getFirstRowNum + 1 to sheet.getLastRowNum)
        .map { case rowNum: Int =>
          val row = sheet.getRow(rowNum)
          Record(
            row.getCell(0).getDateCellValue.toInstant.atZone(ZoneId.systemDefault()).toLocalDate,
            Option(row.getCell(2)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(3)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(4)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(5)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(6)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(7)).map(_.getNumericCellValue.toFloat),
          )
        }
    } match {
      case Success(value) => value
      case Failure(e) => throw e
    }
  }

  def drawWeightGraph(records: Seq[Record]): PlotFile = {
    val relRecords = records.filter(_.weight.isDefined)
    val plot = Plot()
      .withScatter(
        relRecords.map(_.date.toString),
        relRecords.map(_.weight.get.toDouble),
        ScatterOptions()
          .name("Weight over time"))
      .xAxisOptions(AxisOptions().title("Date"))
      .yAxisOptions(AxisOptions().title("Weight (lbs)"))
    draw(plot, "weight", FileOptions(overwrite = true))
  }
}

case class Record(
    date: LocalDate,
    weight: Option[Float],
    fatWeight: Option[Float],
    pctFat: Option[Float],
    pctWater: Option[Float],
    pctBone: Option[Float],
    bmi: Option[Float])
