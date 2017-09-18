package xyz.clieb.bodygraph

import org.apache.poi.ss.usermodel.WorkbookFactory

import java.io.{File, IOException}
import java.nio.file.Path
import java.time.{LocalDate, ZoneId}

import scala.util.{Failure, Success}

import co.theasi.plotly._
import co.theasi.plotly.writer.{FileOptions, PlotFile}
import scopt.OptionParser
import xyz.clieb.bodygraph.Closable._

object Main {
  def main(args: Array[String]): Unit = {
    parser.parse(args, Options()) match {
      case Some(s) => new Main().run(s.path.get.toPath)
      case None =>
    }
  }

  case class Options(path: Option[File] = None)

  val parser = new OptionParser[Options]("body-graphs") {
    head("body-graphs", "0.1")

    arg[File]("<file>").action((x, c) =>
      c.copy(path = Some(x)))

    checkConfig(c =>
      if (c.path.isEmpty) {
        failure("Must specify the weight tracker file")
      } else if (!c.path.get.exists()) {
        failure(s"The input file does not exist: ${c.path.get.getAbsolutePath}")
      } else if (!c.path.get.isFile) {
        failure(s"The input file is not a file: ${c.path.get.getAbsolutePath}")
      } else {
        success
      })
  }
}

class Main {
  def run(path: Path): Unit = {
    val records = readFile(path)
    validateFile(records)
    val outFile = drawWeightGraph(records)
    println(outFile)
  }

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

  def validateFile(records: Seq[Record]): Unit = {
    val errors = (1 until records.size)
      .map(idx => {
        if (records(idx - 1).date.compareTo(records(idx).date) >= 0) {
          Some(s"Date for row ${idx - 1} (${records(idx - 1).date}) is the same or later " +
            s"than the date for row ${idx} (${records(idx).date}")
        } else {
          None
        }
      })
      .filter(_.isDefined)
    if (errors.nonEmpty) {
      throw new IOException(s"Found issues in data read from file: \n${errors.mkString("\n")}")
    }
  }

  def drawWeightGraph(records: Seq[Record]): PlotFile = {
    val relRecords = records.filter(_.weight.isDefined)
    val sevenDayAverage = records.map(record => {
      val lowerBound = record.date.minusDays(7)
      val upperBound = record.date

      val sevenDays = records
        .filter(record => lowerBound.compareTo(record.date) <= 0 && record.date.compareTo(upperBound) <= 0)
        .map(record => record.weight.get)
      (record.date, (sevenDays.sum / sevenDays.size).toDouble)
    })

    val plot = Plot()
      .withScatter(
        relRecords.map(_.date.toString),
        relRecords.map(_.weight.get.toDouble),
        ScatterOptions()
          .name("Weight")
          .mode(ScatterMode.Marker))
      .withScatter(
        sevenDayAverage.map(_._1.toString),
        sevenDayAverage.map(_._2),
        ScatterOptions()
          .name("7-day average"))
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
